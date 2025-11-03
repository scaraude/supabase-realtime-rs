// Module declarations
mod builder;
mod connection;
mod core;
mod state;

// Public API exports
pub use builder::{RealtimeClientBuilder, RealtimeClientOptions};
pub use connection::{ConnectionManager, ConnectionState};
pub use core::RealtimeClient;
pub use state::ClientState;
