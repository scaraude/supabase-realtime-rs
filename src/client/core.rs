use super::{
    ClientState, ConnectionManager, ConnectionState, RealtimeClientBuilder, RealtimeClientOptions,
};
use crate::RealtimeChannel;
use crate::infrastructure::{HeartbeatManager, Timer};
use crate::messaging::MessageRouter;
use crate::types::{RealtimeError, RealtimeMessage, Result};
use crate::websocket::WebSocketFactory;
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

/// The main entry point for interacting with Supabase Realtime.
///
/// `RealtimeClient` manages the WebSocket connection to Supabase Realtime servers,
/// handles automatic reconnection with exponential backoff, and provides channel
/// creation for real-time subscriptions.
///
/// # Example
///
/// ```no_run
/// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = RealtimeClient::new(
///     "wss://your-project.supabase.co/realtime/v1",
///     RealtimeClientOptions {
///         api_key: "your-anon-key".to_string(),
///         ..Default::default()
///     }
/// )?;
///
/// client.connect().await?;
/// // Use the client...
/// client.disconnect().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct RealtimeClient {
    pub(crate) endpoint: String,
    pub(crate) options: RealtimeClientOptions,

    // Connection manager
    pub(crate) connection: Arc<ConnectionManager>,

    // Consolidated mutable state
    pub(crate) state: Arc<RwLock<ClientState>>,
}

impl RealtimeClient {
    /// Creates a new RealtimeClient instance.
    ///
    /// This initializes the client but does not establish a connection. You must call
    /// [`connect()`](Self::connect) to establish the WebSocket connection.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The WebSocket endpoint URL (e.g., `wss://your-project.supabase.co/realtime/v1`)
    /// * `options` - Configuration options including API key and optional settings
    ///
    /// # Returns
    ///
    /// Returns `Ok(RealtimeClient)` if the endpoint is valid, or an error if the URL is malformed.
    ///
    /// # Errors
    ///
    /// Returns [`RealtimeError::UrlParse`](crate::types::RealtimeError::UrlParse) if the endpoint URL cannot be parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = RealtimeClient::new(
    ///     "wss://your-project.supabase.co/realtime/v1",
    ///     RealtimeClientOptions {
    ///         api_key: "your-anon-key".to_string(),
    ///         ..Default::default()
    ///     }
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(endpoint: impl Into<String>, options: RealtimeClientOptions) -> Result<Self> {
        RealtimeClientBuilder::new(endpoint, options).map(|builder| builder.build())
    }

    /// Set connection state and notify watchers
    async fn set_state(&self, new_state: ConnectionState) {
        self.connection.set_state(new_state).await;

        let state = self.state.read().await;
        state.notify_state_change(new_state, state.was_manual_disconnect);
    }

    /// Set manual disconnect flag and notify watchers
    async fn set_manual_disconnect(&self, manual: bool) {
        let mut state = self.state.write().await;
        state.was_manual_disconnect = manual;

        let conn_state = self.connection.state().await;
        state.notify_state_change(conn_state, manual);
    }

    pub async fn resubscribe_all_channels(&self) -> Result<()> {
        let channels = self.state.read().await.channels.clone();
        for channel in channels.iter() {
            if channel.was_joined().await {
                channel.subscribe().await?;
            }
        }
        Ok(())
    }

    pub async fn try_reconnect(&self) -> Result<()> {
        if self.state.read().await.was_manual_disconnect {
            tracing::info!("Manual disconnect detected, will not attempt to reconnect");
            return Ok(());
        }

        let mut timer = Timer::default();
        loop {
            {
                let state = self.connection.state().await;
                if state == ConnectionState::Open || state == ConnectionState::Connecting {
                    tracing::info!(
                        "Already connected or connecting, stopping reconnection attempts"
                    );
                    break;
                }
            }

            tracing::info!("Attempting to reconnect...");
            match self.connect().await {
                Ok(_) => {
                    tracing::info!("Reconnected successfully");
                    self.resubscribe_all_channels().await?;
                    break;
                }
                Err(e) => {
                    tracing::error!("Reconnection attempt failed: {}", e);
                    timer.schedule_timeout().await;
                }
            }
        }
        Ok(())
    }
    /// Establishes a WebSocket connection to the Supabase Realtime server.
    ///
    /// This method opens the WebSocket connection, starts the heartbeat mechanism,
    /// and spawns background tasks for reading messages and maintaining the connection.
    /// If already connected, this method returns immediately without error.
    ///
    /// After connecting successfully, the client will automatically:
    /// - Send periodic heartbeat messages
    /// - Attempt reconnection if the connection drops (unless manually disconnected)
    /// - Route incoming messages to subscribed channels
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The WebSocket handshake fails
    /// - The endpoint URL is invalid
    /// - TLS/SSL negotiation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = RealtimeClient::new(
    ///     "wss://your-project.supabase.co/realtime/v1",
    ///     RealtimeClientOptions {
    ///         api_key: "your-anon-key".to_string(),
    ///         ..Default::default()
    ///     }
    /// )?;
    ///
    /// // Establish connection
    /// client.connect().await?;
    ///
    /// // Now you can create channels and subscribe
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&self) -> Result<()> {
        {
            let state = self.connection.state().await;
            if state == ConnectionState::Open || state == ConnectionState::Connecting {
                return Ok(());
            }
        }
        self.set_state(ConnectionState::Connecting).await;

        // Build WebSocket URL with query parameters
        let url = self.build_endpoint_url()?;
        tracing::info!("Connecting to {}", &self.endpoint);

        // Create WebSocket connection
        let ws_stream = WebSocketFactory::create(url).await?;
        let (write_half, mut read_half) = ws_stream.split();

        // Give write half to ConnectionManager
        self.connection.set_writer(write_half).await;

        // Create message router with Arc to state
        let state_for_router = Arc::clone(&self.state);
        let router = MessageRouter::new_with_state(state_for_router);

        // Spawn read task with router using TaskManager
        let self_cloned = self.clone();
        {
            let mut state = self.state.write().await;
            state.task_manager.spawn(async move {
                tracing::info!("Starting read task");
                while let Some(msg_result) = read_half.next().await {
                    match msg_result {
                        Ok(msg) => {
                            use tokio_tungstenite::tungstenite::Message;

                            match msg {
                                Message::Text(text) => {
                                    tracing::debug!("Received text message: {}", text);
                                    match serde_json::from_str::<RealtimeMessage>(&text) {
                                        Ok(realtime_msg) => {
                                            tracing::debug!(
                                                "Parsed message: topic={}, event={}, payload={}",
                                                realtime_msg.topic,
                                                realtime_msg.event.as_str(),
                                                serde_json::to_string(&realtime_msg.payload)
                                                    .unwrap_or_default()
                                            );
                                            router.route(realtime_msg).await;
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to parse message: {} - Raw: {}",
                                                e,
                                                text
                                            );
                                        }
                                    }
                                }
                                Message::Close(frame) => {
                                    if let Some(close_frame) = frame {
                                        tracing::error!(
                                            "Server closed connection: code={:?}, reason='{}'",
                                            close_frame.code,
                                            close_frame.reason
                                        );
                                    } else {
                                        tracing::warn!(
                                            "Server closed connection without close frame"
                                        );
                                    }
                                    self_cloned.set_state(ConnectionState::Closed).await;
                                    break;
                                }
                                Message::Ping(data) => {
                                    tracing::debug!("Received ping ({} bytes)", data.len());
                                }
                                Message::Pong(data) => {
                                    tracing::debug!("Received pong ({} bytes)", data.len());
                                }
                                Message::Binary(data) => {
                                    tracing::warn!(
                                        "Received unexpected binary message ({} bytes)",
                                        data.len()
                                    );
                                }
                                Message::Frame(_) => {
                                    tracing::debug!("Received raw frame (internal)");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("WebSocket read error: {}", e);
                            self_cloned.set_state(ConnectionState::Closed).await;
                            break;
                        }
                    }
                }
                tracing::info!("Read task finished");
            });
        }

        // Spawn heartbeat task using HeartbeatManager
        let heartbeat_interval = self.options.heartbeat_interval.unwrap_or(25_000);

        let heartbeat_manager =
            HeartbeatManager::new(Arc::downgrade(&self.connection), Arc::clone(&self.state))
                .with_interval(std::time::Duration::from_millis(heartbeat_interval));

        heartbeat_manager.spawn_on(&self.state).await;

        self.set_manual_disconnect(false).await;
        self.set_state(ConnectionState::Open).await;

        tracing::info!("Connected to WebSocket server");
        Ok(())
    }

    /// Creates or retrieves a channel for real-time subscriptions.
    ///
    /// Channels are the primary way to subscribe to real-time events. Each channel is identified
    /// by a unique topic string. If a channel with the given topic already exists, this method
    /// returns the existing channel instead of creating a new one.
    ///
    /// # Arguments
    ///
    /// * `topic` - The channel topic (e.g., "room:lobby", "public:todos"). The "realtime:" prefix
    ///   is automatically added.
    /// * `options` - Configuration options for the channel (broadcast settings, presence key, etc.)
    ///
    /// # Returns
    ///
    /// Returns an `Arc<RealtimeChannel>` that can be used to subscribe to events, send broadcasts,
    /// and track presence.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions, RealtimeChannelOptions};
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
    /// // Subscribe to receive events
    /// channel.subscribe().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn channel(
        &self,
        topic: &str,
        options: crate::channel::RealtimeChannelOptions,
    ) -> Arc<RealtimeChannel> {
        let full_topic = format!("realtime:{}", topic);

        let state = self.state.read().await;
        for existing_channel in state.channels.iter() {
            if existing_channel.topic() == full_topic {
                return Arc::clone(existing_channel);
            }
        }
        drop(state);

        let new_channel = Arc::new(RealtimeChannel::new(
            full_topic,
            Arc::new(self.clone()),
            options,
        ));
        self.state
            .write()
            .await
            .channels
            .push(Arc::clone(&new_channel));

        new_channel
    }

    /// Gracefully disconnects from the WebSocket server.
    ///
    /// This method closes the WebSocket connection, aborts all background tasks (heartbeat,
    /// message reading), and marks the disconnect as manual. When disconnected manually, the
    /// client will NOT attempt automatic reconnection.
    ///
    /// To reconnect after a manual disconnect, call [`connect()`](Self::connect) again.
    ///
    /// # Errors
    ///
    /// Returns an error if the WebSocket close handshake fails (rare).
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
    /// // When done, disconnect gracefully
    /// client.disconnect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disconnect(&self) -> Result<()> {
        {
            let state = self.connection.state().await;
            if state == ConnectionState::Closed {
                return Ok(());
            }
        }

        self.set_manual_disconnect(true).await;
        tracing::info!("Disconnecting from WebSocket server");

        // Abort all tasks via TaskManager
        {
            let mut state = self.state.write().await;
            state.task_manager.abort_all();
            state.pending_heartbeat_ref = None;
        }

        // Close connection via ConnectionManager
        self.connection.close().await?;

        tracing::info!("Disconnected from WebSocket server");
        Ok(())
    }

    /// Checks whether the client is currently connected to the server.
    ///
    /// # Returns
    ///
    /// Returns `true` if the WebSocket connection is open, `false` otherwise.
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
    /// if !client.is_connected().await {
    ///     client.connect().await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_connected(&self) -> bool {
        self.connection.is_connected().await
    }

    /// Build the WebSocket endpoint URL with query parameters
    fn build_endpoint_url(&self) -> Result<String> {
        let mut url = Url::parse(&self.endpoint)?;

        // Add required query parameters
        url.query_pairs_mut()
            .append_pair("apikey", &self.options.api_key);

        Ok(url.to_string())
    }

    /// Generate next message reference
    pub async fn make_ref(&self) -> String {
        let mut state = self.state.write().await;
        state.make_ref()
    }

    /// Push a message to the server
    pub async fn push(&self, message: RealtimeMessage) -> Result<()> {
        if !self.is_connected().await {
            return Err(RealtimeError::NotConnected);
        }

        self.connection.send_message(message).await?;
        Ok(())
    }

    /// Get HTTP endpoint URL (for broadcasts)
    pub fn http_endpoint(&self) -> String {
        crate::infrastructure::ws_to_http_endpoint(&self.endpoint)
    }

    /// Get API key
    pub fn api_key(&self) -> &str {
        &self.options.api_key
    }

    /// Get access token
    pub fn access_token(&self) -> Option<&str> {
        self.options.access_token.as_deref()
    }
}
