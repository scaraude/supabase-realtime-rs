use super::{
    Push,
    config::{
        BroadcastConfig, ChannelJoinConfig, JoinPayload, PostgresChangesPayload, PostgresEventType,
        PresenceConfig,
    },
    state::{ChannelState, ChannelStatus, EventBinding},
};
use crate::types::Result;
use crate::{RealtimeMessage, SystemEvent};
use crate::{channel::PostgresChangesFilter, infrastructure::HttpBroadcaster};
use crate::{channel::PresenceMeta, messaging::ChannelEvent};
use crate::{client::RealtimeClient, types::DEFAULT_TIMEOUT};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, mpsc};

/// Configuration options for a realtime channel.
///
/// These options control broadcasting behavior, presence tracking, and access control.
#[derive(Debug, Clone, Default)]
pub struct RealtimeChannelOptions {
    /// Whether to receive your own broadcast messages. Default: `false`.
    pub broadcast_self: bool,
    /// Whether to receive acknowledgments for broadcast messages. Default: `false`.
    pub broadcast_ack: bool,
    /// Unique key for presence tracking. If `Some`, enables presence tracking.
    pub presence_key: Option<String>,
    /// Whether this is a private channel requiring authorization. Default: `false`.
    pub is_private: bool,
}

/// A channel for subscribing to real-time events from Supabase.
///
/// Channels enable you to:
/// - **Subscribe to database changes** (INSERT, UPDATE, DELETE events from Postgres)
/// - **Send and receive broadcasts** (pub/sub messaging between clients)
/// - **Track presence** (who's online, cursor positions, user status)
///
/// Each channel is identified by a topic string. Multiple clients can subscribe to the same
/// topic to receive shared events.
///
/// # Example
///
/// ```no_run
/// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let client = RealtimeClient::new(
/// #     "wss://your-project.supabase.co/realtime/v1",
/// #     RealtimeClientOptions {
/// #         api_key: "your-anon-key".to_string(),
/// #         ..Default::default()
/// #     }
/// # )?;
/// # client.connect().await?;
/// // Create a channel
/// let channel = client.channel("room:lobby", Default::default()).await;
///
/// // Subscribe to the channel
/// channel.subscribe().await?;
///
/// // Now you can listen to events, send broadcasts, track presence, etc.
/// # Ok(())
/// # }
/// ```
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

    /// Registers an event listener for a specific event type.
    ///
    /// This method returns a channel receiver that will receive payloads whenever the specified
    /// event occurs. You can spawn a task to process events asynchronously.
    ///
    /// # Arguments
    ///
    /// * `event` - The event type to listen for (e.g., `ChannelEvent::broadcast("message")`)
    ///
    /// # Returns
    ///
    /// Returns an `mpsc::Receiver<serde_json::Value>` that receives event payloads.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions, ChannelEvent};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = RealtimeClient::new(
    /// #     "wss://your-project.supabase.co/realtime/v1",
    /// #     RealtimeClientOptions {
    /// #         api_key: "your-anon-key".to_string(),
    /// #         ..Default::default()
    /// #     }
    /// # )?;
    /// # client.connect().await?;
    /// let channel = client.channel("room:lobby", Default::default()).await;
    ///
    /// // Listen for broadcast messages
    /// let mut rx = channel.on(ChannelEvent::broadcast("message")).await;
    ///
    /// channel.subscribe().await?;
    ///
    /// // Spawn a task to process events
    /// tokio::spawn(async move {
    ///     while let Some(payload) = rx.recv().await {
    ///         println!("Received message: {:?}", payload);
    ///     }
    /// });
    /// # Ok(())
    /// # }
    /// ```
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

    /// Subscribes to Postgres database changes for a specific table.
    ///
    /// This method listens for INSERT, UPDATE, and DELETE events from your Postgres database,
    /// delivered in real-time via Supabase Realtime.
    ///
    /// **Note**: You must have Row Level Security (RLS) policies configured with SELECT permissions
    /// on the table for the events to be delivered. See Supabase documentation for details.
    ///
    /// # Arguments
    ///
    /// * `filter` - The postgres changes filter specifying schema, table, and event type
    ///
    /// # Returns
    ///
    /// Returns an `mpsc::Receiver<serde_json::Value>` that receives change payloads.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions, PostgresChangesFilter};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = RealtimeClient::new(
    /// #     "wss://your-project.supabase.co/realtime/v1",
    /// #     RealtimeClientOptions {
    /// #         api_key: "your-anon-key".to_string(),
    /// #         ..Default::default()
    /// #     }
    /// # )?;
    /// # client.connect().await?;
    /// let channel = client.channel("schema-db-changes", Default::default()).await;
    ///
    /// // Listen for all changes to the "todos" table
    /// let mut rx = channel.on_postgres_changes(
    ///     PostgresChangesFilter::new("public", "todos")
    /// ).await;
    ///
    /// channel.subscribe().await?;
    ///
    /// tokio::spawn(async move {
    ///     while let Some(change) = rx.recv().await {
    ///         println!("Database change: {:?}", change);
    ///     }
    /// });
    /// # Ok(())
    /// # }
    /// ```
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
        let event_enum = ChannelEvent::parse(event.as_str());
        let state = self.state.read().await;

        for binding in state.bindings.iter() {
            if binding.event == event_enum {
                if event_enum == ChannelEvent::PostgresChanges
                    && let Some(filters) = &binding.filter
                {
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

    /// Subscribes to the channel to start receiving events.
    ///
    /// You must call `subscribe()` after registering event listeners with [`on()`](Self::on)
    /// or [`on_postgres_changes()`](Self::on_postgres_changes). Only after subscribing will
    /// events be delivered to your listeners.
    ///
    /// If the channel is already subscribed or subscribing, this method returns immediately
    /// without error.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not connected
    /// - The subscription message cannot be sent
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions, ChannelEvent};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = RealtimeClient::new(
    /// #     "wss://your-project.supabase.co/realtime/v1",
    /// #     RealtimeClientOptions {
    /// #         api_key: "your-anon-key".to_string(),
    /// #         ..Default::default()
    /// #     }
    /// # )?;
    /// # client.connect().await?;
    /// let channel = client.channel("room:lobby", Default::default()).await;
    ///
    /// // Register listeners BEFORE subscribing
    /// let mut rx = channel.on(ChannelEvent::broadcast("message")).await;
    ///
    /// // Now subscribe to start receiving events
    /// channel.subscribe().await?;
    /// # Ok(())
    /// # }
    /// ```
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

        let join_ref = self.client.make_ref().await;
        let join_message = RealtimeMessage::new(
            self.topic.clone(),
            ChannelEvent::System(SystemEvent::Join),
            serde_json::to_value(&payload)?,
        )
        .with_ref(self.client.make_ref().await)
        .with_join_ref(join_ref.clone());

        tracing::info!("Subscribing to channel: {}", self.topic,);

        self.client.push(join_message).await?;

        let mut state = self.state.write().await;
        state.status = ChannelStatus::Joined;
        state.join_ref = Some(join_ref);

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

    /// Unsubscribes from the channel and stops receiving events.
    ///
    /// After unsubscribing, event listeners will no longer receive payloads. You can
    /// resubscribe later by calling [`subscribe()`](Self::subscribe) again.
    ///
    /// # Errors
    ///
    /// Returns an error if the unsubscribe message cannot be sent.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = RealtimeClient::new(
    /// #     "wss://your-project.supabase.co/realtime/v1",
    /// #     RealtimeClientOptions {
    /// #         api_key: "your-anon-key".to_string(),
    /// #         ..Default::default()
    /// #     }
    /// # )?;
    /// # client.connect().await?;
    /// let channel = client.channel("room:lobby", Default::default()).await;
    /// channel.subscribe().await?;
    ///
    /// // Later, unsubscribe when done
    /// channel.unsubscribe().await?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Sends a broadcast message to all subscribers of this channel.
    ///
    /// Broadcasts are pub/sub messages sent between clients. This is useful for real-time
    /// collaboration features like chat, cursor tracking, or notifications.
    ///
    /// If connected, the message is sent via WebSocket. If disconnected, it automatically
    /// falls back to HTTP POST to ensure delivery.
    ///
    /// # Arguments
    ///
    /// * `event` - The broadcast event type (e.g., `ChannelEvent::broadcast("message")`)
    /// * `payload` - The message payload as JSON
    ///
    /// # Errors
    ///
    /// Returns an error if both WebSocket and HTTP delivery fail.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions, ChannelEvent};
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = RealtimeClient::new(
    /// #     "wss://your-project.supabase.co/realtime/v1",
    /// #     RealtimeClientOptions {
    /// #         api_key: "your-anon-key".to_string(),
    /// #         ..Default::default()
    /// #     }
    /// # )?;
    /// # client.connect().await?;
    /// let channel = client.channel("room:lobby", Default::default()).await;
    /// channel.subscribe().await?;
    ///
    /// // Send a broadcast message
    /// channel.send(
    ///     ChannelEvent::broadcast("message"),
    ///     json!({"text": "Hello, world!", "user": "Alice"})
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
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

    pub async fn presence_list(&self) -> Vec<(String, Vec<PresenceMeta>)> {
        let state = self.state.read().await;
        state
            .presence
            .list()
            .into_iter()
            .map(|(user_id, metas)| (user_id.clone(), metas.clone()))
            .collect()
    }

    /// Tracks your presence on this channel, broadcasting metadata to other subscribers.
    ///
    /// When you track presence, all other subscribers will receive a `presence_diff` event
    /// with your metadata. You can update your presence by calling `track()` again with
    /// new metadata - this replaces the previous state for your connection.
    ///
    /// Common use cases:
    /// - Online/offline status
    /// - Cursor positions in collaborative editing
    /// - "User is typing..." indicators
    /// - Active users list
    ///
    /// # Arguments
    ///
    /// * `metadata` - Arbitrary JSON metadata to broadcast (status, username, cursor position, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The channel is not subscribed
    /// - The message cannot be sent
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = RealtimeClient::new(
    /// #     "wss://your-project.supabase.co/realtime/v1",
    /// #     RealtimeClientOptions {
    /// #         api_key: "your-anon-key".to_string(),
    /// #         ..Default::default()
    /// #     }
    /// # )?;
    /// # client.connect().await?;
    /// let channel = client.channel("room:lobby", Default::default()).await;
    /// channel.subscribe().await?;
    ///
    /// // Track your presence
    /// channel.track(json!({
    ///     "user": "Alice",
    ///     "status": "online",
    ///     "cursor_x": 100,
    ///     "cursor_y": 200
    /// })).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn track(self: &Arc<Self>, metadata: serde_json::Value) -> Result<()> {
        let payload = serde_json::json!({
            "type": "presence",
            "event" : "track",
            "payload": metadata,
        });

        self.push("presence", payload).send().await
    }

    /// Stops tracking your presence on this channel.
    ///
    /// When you untrack, all other subscribers will receive a `presence_diff` event indicating
    /// you have left. This is useful for graceful cleanup when a user disconnects or leaves
    /// a collaborative session.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The channel is not subscribed
    /// - The message cannot be sent
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = RealtimeClient::new(
    /// #     "wss://your-project.supabase.co/realtime/v1",
    /// #     RealtimeClientOptions {
    /// #         api_key: "your-anon-key".to_string(),
    /// #         ..Default::default()
    /// #     }
    /// # )?;
    /// # client.connect().await?;
    /// let channel = client.channel("room:lobby", Default::default()).await;
    /// channel.subscribe().await?;
    /// channel.track(json!({"user": "Alice", "status": "online"})).await?;
    ///
    /// // Later, stop tracking presence
    /// channel.untrack().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn untrack(self: &Arc<Self>) -> Result<()> {
        let payload = serde_json::json!({
            "type": "presence",
            "event": "untrack",
        });

        self.push("presence", payload).send().await
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
