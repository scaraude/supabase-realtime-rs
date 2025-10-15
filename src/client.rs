use crate::RealtimeChannel;
use crate::timer::Timer;
use crate::types::{ConnectionState, RealtimeError, RealtimeMessage, Result};
use crate::websocket::WebSocketFactory;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{RwLock, watch};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
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
            state: Arc::new(RwLock::new(ConnectionState::Closed)),
            ref_counter: Arc::new(RwLock::new(0)),
            channels: Arc::new(RwLock::new(Vec::new())),
            write_tx: Arc::new(RwLock::new(None)),
            read_task: Arc::new(RwLock::new(None)),
            write_task: Arc::new(RwLock::new(None)),
            pending_heartbeat_ref: Arc::new(RwLock::new(None)),
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
                let (state, was_manual) = *rx.borrow();

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
    state: Arc<RwLock<ConnectionState>>,
    ref_counter: Arc<RwLock<u64>>,

    channels: Arc<RwLock<Vec<Arc<crate::channel::RealtimeChannel>>>>,

    write_tx: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
    read_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    write_task: Arc<RwLock<Option<JoinHandle<()>>>>,

    pending_heartbeat_ref: Arc<RwLock<Option<String>>>,
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
        *self.state.write().await = new_state;

        let was_manual = *self.was_manual_disconnect.read().await;
        if let Some(tx) = self.state_change_tx.read().await.as_ref() {
            let _ = tx.send((new_state, was_manual));
        }
    }

    /// Set manual disconnect flag and notify watchers
    async fn set_manual_disconnect(&self, manual: bool) {
        *self.was_manual_disconnect.write().await = manual;

        let state = *self.state.read().await;
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
                let state = self.state.read().await;
                if *state == ConnectionState::Open || *state == ConnectionState::Connecting {
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
            let state = self.state.read().await;
            if *state == ConnectionState::Open || *state == ConnectionState::Connecting {
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

        let (tx, mut rx) = mpsc::channel::<Message>(100);

        let pending_heartbeat_ref = Arc::clone(&self.pending_heartbeat_ref);
        let channels = Arc::clone(&self.channels);

        let self_cloned = self.clone();
        let read_task = tokio::spawn(async move {
            while let Some(msg_result) = read_half.next().await {
                match msg_result {
                    Ok(msg) => {
                        tracing::debug!("Received message: {:?}", msg);
                        if let Message::Text(text) = msg {
                            match serde_json::from_str::<RealtimeMessage>(&text) {
                                Ok(realtime_msg) => {
                                    // Handle heartbeat response
                                    if realtime_msg.topic == "phoenix"
                                        && (realtime_msg.event == "phx_reply"
                                            || realtime_msg.event == "heartbeat")
                                    // TODO: remove for supabase application
                                    {
                                        if let Some(ref msg_ref) = realtime_msg.r#ref {
                                            let pending = pending_heartbeat_ref.read().await;
                                            if pending.as_ref() == Some(msg_ref) {
                                                drop(pending);
                                                *pending_heartbeat_ref.write().await = None;
                                                tracing::debug!(
                                                    "Received heartbeat ack for ref {}",
                                                    msg_ref
                                                );
                                            }
                                        }
                                    }
                                    let channels = channels.read().await;
                                    for channel in channels.iter() {
                                        if channel.topic() == realtime_msg.topic {
                                            channel
                                                ._trigger(
                                                    &realtime_msg.event,
                                                    realtime_msg.payload.clone(),
                                                )
                                                .await;
                                        }
                                    }
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

        let write_task = tokio::spawn(async move {
            let mut write_half = write_half;
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write_half.send(msg).await {
                    tracing::error!("WebSocket write error: {}", e);
                    break;
                }
            }
            tracing::info!("Write task finished");
        });

        *self.write_tx.write().await = Some(tx);
        *self.read_task.write().await = Some(read_task);
        *self.write_task.write().await = Some(write_task);

        let heartbeat_interval = self.options.heartbeat_interval.unwrap_or(25_000);
        // Clone the Arc references we need
        let pending_heartbeat_ref = Arc::clone(&self.pending_heartbeat_ref);
        let write_tx_arc = Arc::clone(&self.write_tx);
        let ref_counter = Arc::clone(&self.ref_counter);
        let state = Arc::clone(&self.state);
        let client_for_heartbeat = self.clone();

        let heartbeat_task = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(heartbeat_interval));
            loop {
                interval.tick().await;
                if *state.read().await != ConnectionState::Open {
                    break;
                }

                let pending = pending_heartbeat_ref.read().await;
                if pending.is_some() {
                    tracing::warn!("Heartbeat timeout - closing connection");
                    drop(pending);
                    // Close the write channel to trigger connection closure
                    *write_tx_arc.write().await = None;
                    // Set state to Closed which will trigger reconnection watcher
                    client_for_heartbeat
                        .set_state(ConnectionState::Closed)
                        .await;
                    break;
                }
                drop(pending);

                let mut counter = ref_counter.write().await;
                *counter += 1;
                let heartbeat_ref = counter.to_string();
                drop(counter);

                *pending_heartbeat_ref.write().await = Some(heartbeat_ref.clone());

                let tx_guard = write_tx_arc.read().await;
                if let Some(tx) = tx_guard.as_ref() {
                    let heartbeat_message = RealtimeMessage {
                        topic: "phoenix".to_string(),
                        event: "heartbeat".to_string(),
                        payload: serde_json::json!({}),
                        r#ref: Some(heartbeat_ref.clone()),
                        join_ref: None,
                    };
                    let json = serde_json::to_string(&heartbeat_message).unwrap();
                    let ws_message = Message::Text(json.into());

                    if tx.send(ws_message).await.is_err() {
                        tracing::error!("Failed to send heartbeat message");
                        break;
                    }
                    tracing::debug!("Sent heartbeat with ref {}", heartbeat_ref);
                } else {
                    break;
                }
            }
            tracing::info!("Heartbeat task finished");
        });

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
            let state = self.state.read().await;
            if *state == ConnectionState::Closed {
                return Ok(());
            }
        }

        self.set_state(ConnectionState::Closing).await;
        self.set_manual_disconnect(true).await;

        tracing::info!("Disconnecting from WebSocket server");

        {
            let mut write_tx = self.write_tx.write().await;
            *write_tx = None;
        }

        {
            let mut read_task = self.read_task.write().await;
            if let Some(task) = read_task.take() {
                task.abort();
            }
        }

        {
            let mut write_task = self.write_task.write().await;
            if let Some(task) = write_task.take() {
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
        self.set_state(ConnectionState::Closed).await;
        tracing::info!("Disconnected from WebSocket server");
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Open
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

        let tx = self.write_tx.read().await;
        let tx = tx.as_ref().ok_or(RealtimeError::NotConnected)?;

        let json = serde_json::to_string(&message)?;

        let ws_message = Message::Text(json.into());

        tx.send(ws_message)
            .await
            .map_err(|e| RealtimeError::Connection(format!("Failed to send message: {}", e)))?;
        tracing::debug!("Pushed message: {:?}", message);

        Ok(())
    }

    pub fn http_endpoint(&self) -> Result<String> {
        let url = self
            .endpoint
            .replace("ws://", "http://")
            .replace("wss://", "https://");
        Ok(url.split('?').next().unwrap_or(&url).to_string())
    }

    pub fn api_key(&self) -> &str {
        &self.options.api_key
    }

    pub fn access_token(&self) -> Option<&str> {
        self.options.access_token.as_deref()
    }
}
