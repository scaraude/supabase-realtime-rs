use crate::types::{RealtimeError, RealtimeMessage, Result, ConnectionState};
use crate::websocket::WebSocketFactory;
use std::sync::Arc;
use tokio::sync::RwLock;
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

pub struct RealtimeClient {
    endpoint: String,
    options: RealtimeClientOptions,
    state: Arc<RwLock<ConnectionState>>,
    ref_counter: Arc<RwLock<u64>>,
}

impl RealtimeClient {
    pub fn new(endpoint: impl Into<String>, options: RealtimeClientOptions) -> Result<Self> {
        let endpoint = endpoint.into();

        // Validate API key is provided
        if options.api_key.is_empty() {
            return Err(RealtimeError::Auth("API key is required".to_string()));
        }

        Ok(Self {
            endpoint,
            options,
            state: Arc::new(RwLock::new(ConnectionState::Closed)),
            ref_counter: Arc::new(RwLock::new(0)),
        })
    }

    /// Connect to the WebSocket server
    pub async fn connect(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if *state == ConnectionState::Open || *state == ConnectionState::Connecting {
            return Ok(());
        }

        *state = ConnectionState::Connecting;

        // Build WebSocket URL with query parameters
        let url = self.build_endpoint_url()?;

        tracing::info!("Connecting to {}", url);

        // TODO: Implement WebSocket connection
        // This is a placeholder for the actual WebSocket connection logic

        *state = ConnectionState::Open;

        Ok(())
    }

    /// Disconnect from the WebSocket server
    pub async fn disconnect(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if *state == ConnectionState::Closed {
            return Ok(());
        }

        *state = ConnectionState::Closing;

        // TODO: Implement WebSocket disconnection

        *state = ConnectionState::Closed;

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

        // TODO: Implement message pushing through WebSocket
        tracing::debug!("Pushing message: {:?}", message);

        Ok(())
    }
}
