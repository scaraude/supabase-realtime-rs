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

pub mod client;
pub mod channel;
pub mod presence;
pub mod push;
pub mod timer;
pub mod types;
pub mod websocket;

pub use client::{RealtimeClient, RealtimeClientOptions};
pub use channel::{RealtimeChannel, RealtimeChannelOptions};
pub use presence::RealtimePresence;
pub use types::{RealtimeError, RealtimeMessage};
