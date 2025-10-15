use crate::types::{error::Result, message::RealtimeMessage};
use futures::stream::SplitSink;
use futures::SinkExt;
use serde_json;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Closed,
    Connecting,
    Open,
    Closing,
}

pub struct ConnectionManager {
    ws_write: Arc<RwLock<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    state: Arc<RwLock<ConnectionState>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            ws_write: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(ConnectionState::Closed)),
        }
    }

    /// Sets the WebSocket write sink (called after successful connection)
    pub async fn set_writer(
        &self,
        writer: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    ) {
        let mut ws = self.ws_write.write().await;
        *ws = Some(writer);
    }

    /// Gets the current connection state
    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    /// Sets the connection state
    pub async fn set_state(&self, new_state: ConnectionState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }

    /// Checks if currently connected
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Open
    }

    /// Sends a message through the WebSocket connection
    pub async fn send_message(&self, msg: RealtimeMessage) -> Result<()> {
        let json = serde_json::to_string(&msg)?;
        let message = Message::Text(json.into());

        let mut ws_guard = self.ws_write.write().await;
        if let Some(ws) = ws_guard.as_mut() {
            ws.send(message).await?;
        }

        Ok(())
    }

    /// Closes the WebSocket connection gracefully
    pub async fn close(&self) -> Result<()> {
        self.set_state(ConnectionState::Closing).await;

        let mut ws_guard = self.ws_write.write().await;
        if let Some(ws) = ws_guard.as_mut() {
            ws.close().await?;
        }
        *ws_guard = None;

        self.set_state(ConnectionState::Closed).await;

        Ok(())
    }

    /// Clears the writer (used during disconnect)
    pub async fn clear_writer(&self) {
        let mut ws = self.ws_write.write().await;
        *ws = None;
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
