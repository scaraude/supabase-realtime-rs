use thiserror::Error;

#[derive(Error, Debug)]
pub enum RealtimeError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Channel error: {0}")]
    Channel(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Timeout error")]
    Timeout,

    #[error("Not connected")]
    NotConnected,
}

pub type Result<T> = std::result::Result<T, RealtimeError>;
