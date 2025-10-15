// Module declarations
mod builder;
mod client;
mod connection;
mod state;

// Public API exports
pub use builder::{RealtimeClientBuilder, RealtimeClientOptions};
pub use client::RealtimeClient;
pub use connection::{ConnectionManager, ConnectionState};
pub use state::ClientState;
