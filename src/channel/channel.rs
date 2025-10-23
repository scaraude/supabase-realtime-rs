use super::{
    Push,
    config::{
        BroadcastConfig, ChannelJoinConfig, JoinPayload, PostgresChangesPayload, PostgresEventType,
        PresenceConfig,
    },
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
        payload: &PostgresChangesPayload,
    ) -> bool {
        for (key, value) in filter.iter() {
            match key.as_str() {
                "event" => {
                    // "*" matches all events
                    if value == "*" {
                        continue;
                    }
                    // Map filter "event" key to payload.data.type field
                    let event_str = match payload.data.event_type {
                        PostgresEventType::Insert => "INSERT",
                        PostgresEventType::Update => "UPDATE",
                        PostgresEventType::Delete => "DELETE",
                    };
                    if event_str != value {
                        return false;
                    }
                }
                "schema" => {
                    if &payload.data.schema != value {
                        return false;
                    }
                }
                "table" => {
                    if &payload.data.table != value {
                        return false;
                    }
                }
                _ => {
                    // Unknown filter key - could be a custom filter field
                    return false;
                }
            }
        }
        true
    }
    /// Internal method to trigger events to registered listeners
    pub(crate) async fn _trigger(&self, event: ChannelEvent, payload: serde_json::Value) {
        let event_enum = ChannelEvent::from_str(event.as_str());
        let state = self.state.read().await;

        for binding in state.bindings.iter() {
            if binding.event == event_enum {
                if event_enum == ChannelEvent::PostgresChanges {
                    if let Some(filters) = &binding.filter {
                        // Deserialize payload to typed struct for filtering
                        match serde_json::from_value::<PostgresChangesPayload>(payload.clone()) {
                            Ok(typed_payload) => {
                                if !self.matches_postgres_filter(filters, &typed_payload) {
                                    continue;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to deserialize postgres_changes payload: {}. Skipping filter.",
                                    e
                                );
                                continue;
                            }
                        }
                    }
                }
                if let Err(e) = binding.sender.send(payload.clone()).await {
                    tracing::warn!(
                        "Failed to send event '{}' to listener: {}. Channel may be closed or full.",
                        event.as_str(),
                        e
                    );
                }
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

        // Collect postgres_changes filters from bindings
        let postgres_changes_configs: Vec<crate::channel::config::PostgresChangesConfig> = state
            .bindings
            .iter()
            .filter(|b| b.event == ChannelEvent::PostgresChanges)
            .filter_map(|b| {
                b.filter
                    .as_ref()
                    .map(|filter| crate::channel::config::PostgresChangesConfig {
                        event: filter
                            .get("event")
                            .cloned()
                            .unwrap_or_else(|| "*".to_string()),
                        schema: filter
                            .get("schema")
                            .cloned()
                            .unwrap_or_else(|| "public".to_string()),
                        table: filter.get("table").cloned(),
                        filter: filter.get("filter").cloned(),
                    })
            })
            .collect();

        drop(state);

        // Build typed join payload per Supabase protocol
        // Reference: https://supabase.com/docs/guides/realtime/protocol
        let payload = JoinPayload {
            config: ChannelJoinConfig {
                broadcast: BroadcastConfig {
                    self_: self.options.broadcast_self,
                    ack: self.options.broadcast_ack,
                },
                presence: PresenceConfig {
                    key: self.options.presence_key.clone(),
                    enabled: self.options.presence_key.is_some(),
                },
                is_private: self.options.is_private,
                postgres_changes: if postgres_changes_configs.is_empty() {
                    None
                } else {
                    Some(postgres_changes_configs)
                },
            },
            access_token: self.client.access_token().map(|s| s.to_string()),
        };

        let join_message = RealtimeMessage::new(
            self.topic.clone(),
            ChannelEvent::System(SystemEvent::Join),
            serde_json::to_value(&payload)?,
        )
        .with_ref(self.client.make_ref().await)
        .with_join_ref(self.client.make_ref().await);

        tracing::info!("Subscribing to channel: {}", self.topic,);

        self.client.push(join_message).await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_payload_serialization() {
        // Reference: https://supabase.com/docs/guides/realtime/protocol
        // Verify that JoinPayload serializes to correct format per Supabase docs

        use crate::channel::config::*;

        let payload = JoinPayload {
            config: ChannelJoinConfig {
                broadcast: BroadcastConfig {
                    self_: true,
                    ack: false,
                },
                presence: PresenceConfig {
                    key: Some("user-123".to_string()),
                    enabled: true,
                },
                is_private: false,
                postgres_changes: None,
            },
            access_token: None,
        };

        let json = serde_json::to_value(&payload).unwrap();

        // Verify structure matches Supabase protocol
        assert!(json.get("config").is_some(), "Missing config field");

        let config = json.get("config").unwrap();
        assert!(
            config.get("broadcast").is_some(),
            "Missing broadcast config"
        );
        assert!(config.get("presence").is_some(), "Missing presence config");
        assert_eq!(config.get("private").unwrap(), false, "Wrong private value");

        let broadcast = config.get("broadcast").unwrap();
        assert_eq!(broadcast.get("self").unwrap(), true, "Wrong broadcast.self");
        assert_eq!(broadcast.get("ack").unwrap(), false, "Wrong broadcast.ack");

        let presence = config.get("presence").unwrap();
        assert_eq!(
            presence.get("key").unwrap(),
            "user-123",
            "Wrong presence key"
        );
        assert_eq!(
            presence.get("enabled").unwrap(),
            true,
            "Wrong presence enabled"
        );

        println!("✅ JoinPayload serializes correctly per Supabase protocol");
        println!("📋 {}", serde_json::to_string_pretty(&json).unwrap());
    }

    #[test]
    fn test_message_with_ref_fields() {
        // Verify that ref and join_ref can be added to messages
        let message = RealtimeMessage::new(
            "realtime:test".to_string(),
            ChannelEvent::System(SystemEvent::Join),
            serde_json::json!({}),
        )
        .with_ref("1".to_string())
        .with_join_ref("1".to_string());

        assert!(message.r#ref.is_some(), "ref should be set");
        assert!(message.join_ref.is_some(), "join_ref should be set");
        assert_eq!(message.r#ref.as_ref().unwrap(), "1");
        assert_eq!(message.join_ref.as_ref().unwrap(), "1");

        println!("✅ Message with ref fields works correctly");
    }
}
