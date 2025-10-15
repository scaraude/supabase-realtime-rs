//! # Supabase Realtime Rust
//!
//! An unofficial Rust client for Supabase Realtime (Phoenix Channels WebSocket protocol).
//!
//! ## Example
//!
//! ```no_run
//! use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = RealtimeClient::new(
//!         "wss://your-project.supabase.co/realtime/v1",
//!         RealtimeClientOptions {
//!             api_key: "your-anon-key".to_string(),
//!             ..Default::default()
//!         }
//!     )?;
//!
//!     client.connect().await?;
//!     Ok(())
//! }
//! ```

pub mod channel;
pub mod client;
pub mod infrastructure;
pub mod messaging;
pub mod types;
pub mod websocket;

pub use channel::{ChannelState, RealtimeChannel, RealtimeChannelOptions};
pub use client::{
    ClientState, ConnectionManager, ConnectionState, RealtimeClient, RealtimeClientOptions,
};
pub use messaging::{ChannelEvent, PostgresChangeFilter, PostgresChangeType, SystemEvent};
pub use types::{RealtimeError, RealtimeMessage};
