use crate::types::error::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream as TungsteniteStream, connect_async};

pub struct WebSocketFactory;

impl WebSocketFactory {
    /// Create a new WebSocket connection
    pub async fn create(url: String) -> Result<TungsteniteStream<MaybeTlsStream<TcpStream>>> {
        tracing::debug!("Creating WebSocket connection");
        let (ws_stream, _) = connect_async(url).await?;
        Ok(ws_stream)
    }
}
