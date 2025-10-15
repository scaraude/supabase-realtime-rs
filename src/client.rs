use crate::connection::{ConnectionManager, ConnectionState};
use crate::heartbeat::HeartbeatManager;
use crate::router::MessageRouter;
use crate::RealtimeChannel;
use crate::timer::Timer;
use crate::types::{RealtimeError, RealtimeMessage, Result};
use crate::websocket::WebSocketFactory;
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tokio::task::JoinHandle;
use url::Url;

#[derive(Debug, Clone)]
pub struct RealtimeClientOptions {
    pub api_key: String,
    pub timeout: Option<u64>,
    pub heartbeat_interval: Option<u64>,
    pub access_token: Option<String>,
}

impl Default for RealtimeClientOptions {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            timeout: None,
            heartbeat_interval: None,
            access_token: None,
        }
    }
}

/// Builder for RealtimeClient that handles initialization
pub struct RealtimeClientBuilder {
    endpoint: String,
    options: RealtimeClientOptions,
}

impl RealtimeClientBuilder {
    /// Create a new builder
    pub fn new(endpoint: impl Into<String>, options: RealtimeClientOptions) -> Result<Self> {
        let endpoint = endpoint.into();

        // Validate API key is provided
        if options.api_key.is_empty() {
            return Err(RealtimeError::Auth("API key is required".to_string()));
        }

        Ok(Self { endpoint, options })
    }

    /// Build the client and spawn background tasks
    pub fn build(self) -> RealtimeClient {
        let client = RealtimeClient {
            endpoint: self.endpoint,
            options: self.options,
            connection: Arc::new(ConnectionManager::new()),
            ref_counter: Arc::new(RwLock::new(0)),
            channels: Arc::new(RwLock::new(Vec::new())),
            pending_heartbeat_ref: Arc::new(RwLock::new(None)),
            read_task: Arc::new(RwLock::new(None)),
            heartbeat_task: Arc::new(RwLock::new(None)),
            was_manual_disconnect: Arc::new(RwLock::new(false)),
            state_change_tx: Arc::new(RwLock::new(None)),
            reconnect_task: Arc::new(RwLock::new(None)),
        };

        // Initialize state watcher channel
        let (state_tx, state_rx) = watch::channel((ConnectionState::Closed, false));

        // Store the sender (we can't use async in build, so we'll use blocking)
        // This is safe because we're just setting an Option from None to Some
        if let Ok(mut tx) = client.state_change_tx.try_write() {
            *tx = Some(state_tx);
        }

        // Spawn reconnection watcher task
        let client_for_watcher = client.clone();
        let watcher_task = tokio::spawn(async move {
            let mut rx = state_rx;

            while rx.changed().await.is_ok() {
                let (state, was_manual) = *rx.borrow_and_update();

                // Reconnect if closed/disconnected AND not manual
                if matches!(state, ConnectionState::Closed) && !was_manual {
                    tracing::info!("State watcher detected disconnect, attempting reconnection...");

                    if let Err(e) = client_for_watcher.try_reconnect().await {
                        tracing::error!("Reconnection watcher failed: {}", e);
                    }
                }
            }
            tracing::info!("Reconnection watcher task finished");
        });

        // Store the watcher task
        if let Ok(mut task) = client.reconnect_task.try_write() {
            *task = Some(watcher_task);
        }

        client
    }
}

#[derive(Clone)]
pub struct RealtimeClient {
    endpoint: String,
    options: RealtimeClientOptions,
    ref_counter: Arc<RwLock<u64>>,

    // New managers
    connection: Arc<ConnectionManager>,
    pending_heartbeat_ref: Arc<RwLock<Option<String>>>, // Shared with heartbeat manager

    channels: Arc<RwLock<Vec<Arc<crate::channel::RealtimeChannel>>>>,

    read_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    heartbeat_task: Arc<RwLock<Option<JoinHandle<()>>>>,

    was_manual_disconnect: Arc<RwLock<bool>>,
    state_change_tx: Arc<RwLock<Option<watch::Sender<(ConnectionState, bool)>>>>,
    reconnect_task: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl RealtimeClient {
    /// Create a new RealtimeClient using the builder pattern
    ///
    /// # Example
    /// ```
    /// let client = RealtimeClient::new(endpoint, options)?.build();
    /// ```
    pub fn new(
        endpoint: impl Into<String>,
        options: RealtimeClientOptions,
    ) -> Result<RealtimeClientBuilder> {
        RealtimeClientBuilder::new(endpoint, options)
    }

    /// Set connection state and notify watchers
    async fn set_state(&self, new_state: ConnectionState) {
        self.connection.set_state(new_state.clone()).await;

        let was_manual = *self.was_manual_disconnect.read().await;
        if let Some(tx) = self.state_change_tx.read().await.as_ref() {
            let _ = tx.send((new_state, was_manual));
        }
    }

    /// Set manual disconnect flag and notify watchers
    async fn set_manual_disconnect(&self, manual: bool) {
        *self.was_manual_disconnect.write().await = manual;

        let state = self.connection.state().await;
        if let Some(tx) = self.state_change_tx.read().await.as_ref() {
            let _ = tx.send((state, manual));
        }
    }

    pub async fn resubscribe_all_channels(&self) -> Result<()> {
        let channels = self.channels.read().await;
        for channel in channels.iter() {
            if channel.was_joined().await {
                channel.subscribe().await?;
            }
        }
        Ok(())
    }

    pub async fn try_reconnect(&self) -> Result<()> {
        if *self.was_manual_disconnect.read().await {
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
    /// Connect to the WebSocket server
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
        tracing::info!("Connecting to {}", url);

        // Create WebSocket connection
        let ws_stream = WebSocketFactory::create(url).await?;
        let (write_half, mut read_half) = ws_stream.split();

        // Give write half to ConnectionManager
        self.connection.set_writer(write_half).await;

        // Create message router
        let router = MessageRouter::new(
            Arc::clone(&self.channels),
            Arc::clone(&self.pending_heartbeat_ref),
        );

        // Spawn read task with router
        let self_cloned = self.clone();
        let read_task = tokio::spawn(async move {
            while let Some(msg_result) = read_half.next().await {
                match msg_result {
                    Ok(msg) => {
                        tracing::debug!("Received message: {:?}", msg);
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                            match serde_json::from_str::<RealtimeMessage>(&text) {
                                Ok(realtime_msg) => {
                                    router.route(realtime_msg).await;
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse message: {}", e);
                                }
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

        *self.read_task.write().await = Some(read_task);

        // Spawn heartbeat task using HeartbeatManager
        let heartbeat_interval = self.options.heartbeat_interval.unwrap_or(25_000);
        let ref_counter = Arc::clone(&self.ref_counter);

        let heartbeat_manager = HeartbeatManager::new_with_counter(
            Arc::downgrade(&self.connection),
            ref_counter,
            Arc::clone(&self.pending_heartbeat_ref),
        )
        .with_interval(std::time::Duration::from_millis(heartbeat_interval));

        let heartbeat_task = heartbeat_manager.spawn();
        *self.heartbeat_task.write().await = Some(heartbeat_task);

        self.set_manual_disconnect(false).await;
        self.set_state(ConnectionState::Open).await;

        tracing::info!("Connected to WebSocket server");
        Ok(())
    }

    pub async fn channel(
        &self,
        topic: &str,
        options: crate::channel::RealtimeChannelOptions,
    ) -> Arc<RealtimeChannel> {
        let full_topic = format!("realtime:{}", topic);

        let channels = self.channels.read().await;
        for existing_channel in channels.iter() {
            if existing_channel.topic() == full_topic {
                return Arc::clone(existing_channel);
            }
        }
        drop(channels);

        let new_channel = Arc::new(RealtimeChannel::new(
            full_topic,
            Arc::new(self.clone()),
            options,
        ));
        self.channels.write().await.push(Arc::clone(&new_channel));

        new_channel
    }

    /// Disconnect from the WebSocket server
    pub async fn disconnect(&self) -> Result<()> {
        {
            let state = self.connection.state().await;
            if state == ConnectionState::Closed {
                return Ok(());
            }
        }

        self.set_manual_disconnect(true).await;
        tracing::info!("Disconnecting from WebSocket server");

        // Abort tasks
        {
            let mut read_task = self.read_task.write().await;
            if let Some(task) = read_task.take() {
                task.abort();
            }
        }

        {
            let mut heartbeat_task = self.heartbeat_task.write().await;
            if let Some(task) = heartbeat_task.take() {
                task.abort();
            }
            let mut pending_heartbeat_ref = self.pending_heartbeat_ref.write().await;
            *pending_heartbeat_ref = None;
        }

        // Close connection via ConnectionManager
        self.connection.close().await?;

        tracing::info!("Disconnected from WebSocket server");
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.connection.is_connected().await
    }

    /// Build the WebSocket endpoint URL with query parameters
    fn build_endpoint_url(&self) -> Result<String> {
        let mut url = Url::parse(&self.endpoint)?;

        // Add required query parameters
        url.query_pairs_mut()
            .append_pair("apikey", &self.options.api_key)
            .append_pair("vsn", "1.0.0");

        Ok(url.to_string())
    }

    /// Generate next message reference
    pub async fn make_ref(&self) -> String {
        let mut counter = self.ref_counter.write().await;
        *counter += 1;
        counter.to_string()
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
        crate::http::ws_to_http_endpoint(&self.endpoint)
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
