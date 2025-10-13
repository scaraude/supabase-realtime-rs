use crate::RealtimeMessage;
use crate::client::RealtimeClient;
use crate::types::{ChannelState, Result};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

struct EventBinding {
    event: String,
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

    pub async fn on(&self, event: &str) -> mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = mpsc::channel(100);
        let binding = EventBinding {
            event: event.to_string(),
            sender: tx,
        };

        self.bindings.write().await.push(binding);

        rx
    }

    pub(crate) async fn _trigger(&self, event: &str, payload: serde_json::Value) {
        let bindings = self.bindings.read().await;
        for binding in bindings.iter() {
            if binding.event == event {
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

    pub fn topic(&self) -> &str {
        &self.topic
    }
}
