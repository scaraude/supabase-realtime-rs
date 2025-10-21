use super::{
    Push,
    state::{ChannelState, ChannelStatus, EventBinding},
};
use crate::messaging::ChannelEvent;
use crate::types::Result;
use crate::{RealtimeMessage, SystemEvent};
use crate::{channel::PostgresChangesFilter, infrastructure::HttpBroadcaster};
use crate::{client::RealtimeClient, types::DEFAULT_TIMEOUT};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, mpsc};

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
    pub(crate) state: Arc<RwLock<ChannelState>>,
    options: RealtimeChannelOptions,
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
            state: Arc::new(RwLock::new(ChannelState::new())),
            options,
        }
    }

    pub async fn was_joined(&self) -> bool {
        self.state.read().await.status == ChannelStatus::Joined
    }

    /// Register an event listener for a specific event type
    pub async fn on(&self, event: impl Into<ChannelEvent>) -> mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = mpsc::channel(100);
        let binding = EventBinding {
            event: event.into(),
            filter: None,
            sender: tx,
        };

        self.state.write().await.bindings.push(binding);

        rx
    }

    pub async fn on_postgres_changes(
        &self,
        filter: PostgresChangesFilter,
    ) -> mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = mpsc::channel(100);
        let binding = EventBinding {
            event: ChannelEvent::PostgresChanges,
            filter: Some(filter.to_hash_map()),
            sender: tx,
        };

        self.state.write().await.bindings.push(binding);

        rx
    }

    fn matches_postgres_filter(
        &self,
        filter: &HashMap<String, String>,
        payload: &serde_json::Value,
    ) -> bool {
        if let Some(payload_obj) = payload.as_object() {
            let Some(payload_data) = payload_obj.get("data") else {
                return false;
            };
            for (key, value) in filter.iter() {
                if let Some(payload_value) = payload_data.get(key) {
                    if payload_value != value {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
    /// Internal method to trigger events to registered listeners
    pub(crate) async fn _trigger(&self, event: ChannelEvent, payload: serde_json::Value) {
        let event_enum = ChannelEvent::from_str(event.as_str());
        let state = self.state.read().await;

        for binding in state.bindings.iter() {
            if binding.event == event_enum {
                if event_enum == ChannelEvent::PostgresChanges {
                    if let Some(filters) = &binding.filter {
                        if !self.matches_postgres_filter(filters, &payload) {
                            continue;
                        }
                    }
                }
                let _ = binding.sender.send(payload.clone()).await;
            };
        }
    }

    /// Subscribe to the channel
    pub async fn subscribe(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if state.status == ChannelStatus::Joined || state.status == ChannelStatus::Joining {
            return Ok(());
        }

        state.status = ChannelStatus::Joining;
        drop(state);

        let join_message = RealtimeMessage::new(
            self.topic.clone(),
            ChannelEvent::System(SystemEvent::Join),
            serde_json::json!({}),
        );

        self.client.push(join_message).await?;

        tracing::info!("Subscribing to channel: {}", self.topic);

        self.state.write().await.status = ChannelStatus::Joined;

        Ok(())
    }

    pub async fn send_http(&self, event: ChannelEvent, payload: serde_json::Value) -> Result<()> {
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

        if state.status == ChannelStatus::Closed {
            return Ok(());
        }

        state.status = ChannelStatus::Leaving;
        drop(state);

        let leave_message = RealtimeMessage::new(
            self.topic.clone(),
            ChannelEvent::System(SystemEvent::Leave),
            serde_json::json!({}),
        );

        self.client.push(leave_message).await?;

        tracing::info!("Unsubscribing from channel: {}", self.topic);

        self.state.write().await.status = ChannelStatus::Closed;

        Ok(())
    }

    pub async fn send(&self, event: ChannelEvent, payload: serde_json::Value) -> Result<()> {
        let is_joined = {
            let state = self.state.read().await;
            state.status == ChannelStatus::Joined
        };

        let is_connected = self.client.is_connected().await;

        if is_joined && is_connected {
            let broadcast_topic = format!("{}:broadcast", self.topic);
            let message = RealtimeMessage::new(
                broadcast_topic,
                event.clone(),
                serde_json::json!({ "type": "broadcast", "payload": payload }),
            );

            self.client.push(message).await?;
            tracing::debug!("Sent broadcast via WebSocket: {}", event.as_str());
            Ok(())
        } else {
            self.send_http(event, payload).await
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub(crate) async fn send_message(&self, message: RealtimeMessage) -> Result<()> {
        self.client.push(message).await
    }

    pub fn push(
        self: &Arc<Self>,
        event: impl Into<ChannelEvent>,
        payload: serde_json::Value,
    ) -> Push {
        let ref_id = uuid::Uuid::new_v4().to_string();
        let event = event.into();
        Push::new(
            event.as_str().to_string(),
            payload,
            ref_id,
            std::time::Duration::from_millis(DEFAULT_TIMEOUT), // 10 seconds
            Arc::clone(self),
        )
    }
}
