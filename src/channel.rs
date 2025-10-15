use crate::client::RealtimeClient;
use crate::event::ChannelEvent;
use crate::http::HttpBroadcaster;
use crate::types::{ChannelState, Result};
use crate::RealtimeMessage;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

struct EventBinding {
    event: ChannelEvent,
    sender: mpsc::Sender<serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct RealtimeChannelOptions {
    pub broadcast_self: bool,
    pub broadcast_ack: bool,
    pub presence_key: Option<String>,
    pub is_private: bool,
}

pub struct RealtimeChannel {
    topic: String,
    client: Arc<RealtimeClient>,
    state: Arc<RwLock<ChannelState>>,
    options: RealtimeChannelOptions,
    bindings: Arc<RwLock<Vec<EventBinding>>>,
}

impl RealtimeChannel {
    pub fn new(
        topic: String,
        client: Arc<RealtimeClient>,
        options: RealtimeChannelOptions,
    ) -> Self {
        Self {
            topic,
            client,
            state: Arc::new(RwLock::new(ChannelState::Closed)),
            options,
            bindings: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn was_joined(&self) -> bool {
        *self.state.read().await == ChannelState::Joined
    }

    /// Register an event listener for a specific event type
    pub async fn on(&self, event: impl Into<ChannelEvent>) -> mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = mpsc::channel(100);
        let binding = EventBinding {
            event: event.into(),
            sender: tx,
        };

        self.bindings.write().await.push(binding);

        rx
    }

    /// Internal method to trigger events to registered listeners
    pub(crate) async fn _trigger(&self, event: &str, payload: serde_json::Value) {
        let event_enum = ChannelEvent::from_str(event);
        let bindings = self.bindings.read().await;
        for binding in bindings.iter() {
            if binding.event == event_enum {
                let _ = binding.sender.send(payload.clone()).await;
            }
        }
    }

    /// Subscribe to the channel
    pub async fn subscribe(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if *state == ChannelState::Joined || *state == ChannelState::Joining {
            return Ok(());
        }

        *state = ChannelState::Joining;
        drop(state);

        let join_message = RealtimeMessage::new(
            self.topic.clone(),
            "phx_join".to_string(),
            serde_json::json!({}),
        );

        self.client.push(join_message).await?;

        tracing::info!("Subscribing to channel: {}", self.topic);

        *self.state.write().await = ChannelState::Joined;

        Ok(())
    }

    pub async fn send_http(&self, event: &str, payload: serde_json::Value) -> Result<()> {
        let broadcaster = HttpBroadcaster::new(
            self.client.http_endpoint(),
            self.client.api_key().to_string(),
            self.client.access_token().map(|s| s.to_string()),
        );

        broadcaster
            .broadcast(&self.topic, event, payload, self.options.is_private)
            .await
    }

    /// Unsubscribe from the channel
    pub async fn unsubscribe(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if *state == ChannelState::Closed {
            return Ok(());
        }

        *state = ChannelState::Leaving;
        drop(state);

        let leave_message = RealtimeMessage::new(
            self.topic.clone(),
            "phx_leave".to_string(),
            serde_json::json!({}),
        );

        self.client.push(leave_message).await?;

        tracing::info!("Unsubscribing from channel: {}", self.topic);

        *self.state.write().await = ChannelState::Closed;

        Ok(())
    }

    pub async fn send(&self, event: &str, payload: serde_json::Value) -> Result<()> {
        let is_joined = {
            let state = self.state.read().await;
            *state == ChannelState::Joined
        };

        let is_connected = self.client.is_connected().await;

        if is_joined && is_connected {
            let message = RealtimeMessage::new(
                self.topic.clone(),
                format!("broadcast:{}", event),
                serde_json::json!({ "type": "broadcast", "event": event, "payload": payload }),
            );

            self.client.push(message).await?;
            tracing::debug!("Sent broadcast via WebSocket: {}", event);
            Ok(())
        } else {
            self.send_http(event, payload).await
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}
