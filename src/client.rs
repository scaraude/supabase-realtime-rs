use crate::types::{RealtimeError, RealtimeMessage, Result, ConnectionState};
use crate::websocket::WebSocketFactory;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;
use futures::stream::SplitSink;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use tokio::net::TcpStream;
use futures::stream::StreamExt;
use futures::sink::SinkExt;

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
    write_tx: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
    read_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    write_task: Arc<RwLock<Option<JoinHandle<()>>>>,
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
            write_tx: Arc::new(RwLock::new(None)),
            read_task: Arc::new(RwLock::new(None)),
            write_task: Arc::new(RwLock::new(None)),
        })
    }

    /// Connect to the WebSocket server
    pub async fn connect(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if *state == ConnectionState::Open || *state == ConnectionState::Connecting {
            return Ok(());
        }

        *state = ConnectionState::Connecting;
        drop(state);
        // Build WebSocket URL with query parameters
        let url = self.build_endpoint_url()?;
        tracing::info!("Connecting to {}", url);

        // Create WebSocket connection
        let ws_stream = WebSocketFactory::create(url).await?;
        let (write_half, mut read_half) = ws_stream.split();

        let (tx, mut rx) = mpsc::channel::<Message>(100);

        let read_task = tokio::spawn(async move {
            while let Some(msg_result) = read_half.next().await {
                match msg_result {
                    Ok(msg) => {
                        tracing::debug!("Received message: {:?}", msg);
                        // Handle incoming messages here
                    }
                    Err(e) => {
                        tracing::error!("WebSocket read error: {}", e);
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

        *self.state.write().await = ConnectionState::Open;

            tracing::info!("Connected to WebSocket server");
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
