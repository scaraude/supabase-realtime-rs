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
pub mod event;
pub mod heartbeat;
pub mod http;
pub mod router;
pub mod task_manager;
pub mod timer;
pub mod types;
pub mod websocket;

pub use channel::{ChannelState, RealtimeChannel, RealtimeChannelOptions};
pub use client::{
    ClientState, ConnectionManager, ConnectionState, RealtimeClient, RealtimeClientOptions,
};
pub use event::{ChannelEvent, PostgresChangeFilter, PostgresChangeType, SystemEvent};
pub use types::{RealtimeError, RealtimeMessage};
