use thiserror::Error;

/// Errors that can occur when using the Supabase Realtime client.
#[derive(Error, Debug)]
pub enum RealtimeError {
    /// WebSocket protocol error (connection failed, invalid frame, etc.)
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    /// General connection error with descriptive message
    #[error("Connection error: {0}")]
    Connection(String),

    /// Authentication or authorization error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Channel-specific error (subscription failed, invalid topic, etc.)
    #[error("Channel error: {0}")]
    Channel(String),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP request error (used for HTTP fallback broadcasts)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// URL parsing error (malformed endpoint URL)
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// Operation timed out (e.g., push message acknowledgment not received)
    #[error("Timeout error")]
    Timeout,

    /// Attempted operation while not connected to the server
    #[error("Not connected")]
    NotConnected,
}

/// Convenience type alias for `Result<T, RealtimeError>`.
pub type Result<T> = std::result::Result<T, RealtimeError>;
