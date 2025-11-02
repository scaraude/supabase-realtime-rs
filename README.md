# Supabase Realtime Rust 🦀

A Rust client for [Supabase Realtime](https://supabase.com/docs/guides/realtime) - Phoenix Channels WebSocket protocol implementation.

> ⚠️ **Work in Progress** - Core WebSocket, channels, and broadcasting are working! Advanced features coming next.

> **Note**: This is an unofficial, community-maintained client. For official clients, see [supabase-community](https://github.com/supabase-community).

## Features

- ✅ Type-safe error handling with `thiserror`
- ✅ Async/await with Tokio
- ✅ WebSocket support with `tokio-tungstenite`
- ✅ Connection management (connect/disconnect)
- ✅ Concurrent read/write tasks
- ✅ Heartbeat mechanism with timeout detection
- ✅ Message serialization/deserialization
- ✅ Message routing and parsing
- ✅ Channel subscriptions (subscribe/unsubscribe)
- ✅ Event listeners with mpsc channels
- ✅ Broadcast messages via WebSocket
- ✅ HTTP fallback for broadcasts when disconnected
- ✅ Automatic reconnection with exponential backoff
- ✅ Manual vs automatic disconnect detection
- ✅ Channel re-subscription after reconnect
- ✅ Push messages with acknowledgments
- ✅ Callback registration for push responses (ok/error/timeout)
- ✅ Timeout mechanism for push messages
- ✅ Postgres changes subscription (basic filtering)
- 🚧 Presence tracking (core types and sync logic implemented)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
supabase-realtime-rs = { git = "https://github.com/Scaraude/supabase-realtime-rs" }
```

Or for local development:

```toml
[dependencies]
supabase-realtime-rs = { path = "../supabase-realtime-rs" }
```

## Usage

```rust
use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RealtimeClient::new(
        "wss://your-project.supabase.co/realtime/v1",
        RealtimeClientOptions {
            api_key: "your-anon-key".to_string(),
            ..Default::default()
        },
    )?;

    client.connect().await?;

    // Your realtime logic here

    client.disconnect().await?;
    Ok(())
}
```

## Examples

Run the examples:

```bash
# Basic connection test
cargo run --example test_connection

# Heartbeat mechanism test
cargo run --example test_heartbeat

# Channel subscription test
cargo run --example test_channel

# Subscribe/unsubscribe test
cargo run --example test_unsubscribe

# Broadcast message test
cargo run --example test_send

# HTTP fallback test
cargo run --example test_http_fallback

# Reconnection infrastructure test
cargo run --example test_reconnection

# Push messages with acknowledgments test
cargo run --example test_push

# Postgres changes (database events) test
cargo run --example test_postgres_changes

# Basic usage example (requires Supabase project)
cargo run --example basic
```

## Project Structure

```
src/
├── lib.rs              # Public API exports
├── client/             # Client module (connection management)
│   ├── builder.rs      # RealtimeClientBuilder with state watcher
│   ├── client.rs       # RealtimeClient - main API
│   ├── connection.rs   # ConnectionManager - WebSocket lifecycle
│   └── state.rs        # ClientState - shared mutable state
├── channel/            # Channel module (subscriptions)
│   ├── channel.rs      # RealtimeChannel implementation
│   └── state.rs        # ChannelState management
├── messaging/          # Message handling
│   ├── event.rs        # ChannelEvent, SystemEvent types
│   └── router.rs       # Message routing logic
├── infrastructure/     # Infrastructure services
│   ├── heartbeat.rs    # Heartbeat mechanism
│   ├── http.rs         # HTTP fallback for broadcasts
│   ├── task_manager.rs # Background task management
│   └── timer.rs        # Reconnection timer with backoff
├── types/              # Core type definitions
│   ├── constants.rs    # Protocol constants
│   ├── error.rs        # Error types
│   └── message.rs      # Message types
└── websocket/          # WebSocket abstraction
    └── factory.rs      # WebSocket factory
```

## Development Roadmap

### Phase 1: Core Infrastructure ✅ COMPLETE
- [x] Project setup
- [x] Type definitions
- [x] Error handling
- [x] Basic client structure

### Phase 2: WebSocket Implementation ✅ COMPLETE
- [x] WebSocket connection (tokio-tungstenite)
- [x] Connection state management
- [x] Concurrent read/write tasks
- [x] Message serialization/deserialization (serde_json)
- [x] Heartbeat mechanism with timeout
- [x] Message routing and parsing

### Phase 3: Heartbeat & Reconnection ✅ COMPLETE
- [x] Heartbeat implementation with timeout
- [x] Heartbeat acknowledgment handling
- [x] Automatic reconnection logic with exponential backoff
- [x] State watcher pattern for disconnect detection
- [x] Manual vs automatic disconnect handling
- [x] Channel re-subscription after reconnect

### Phase 4: Channel Implementation ✅ COMPLETE
- [x] Channel creation (client.channel())
- [x] Subscribe/unsubscribe to channels
- [x] Event listeners with mpsc channels
- [x] Message routing to channels
- [x] Broadcast messages via WebSocket
- [x] HTTP fallback for broadcasts

### Phase 5: Advanced Features (In Progress)
- [x] Push messages with acknowledgments
- [x] Callback registration (ok/error/timeout)
- [x] Timeout mechanism with tokio
- [x] Postgres changes subscription (basic filtering)
- [ ] Presence tracking
- [ ] Access token refresh

### Phase 6: Testing & Polish
- [x] Basic connection tests
- [x] Heartbeat tests
- [x] Channel subscription tests
- [x] Broadcast tests
- [x] Reconnection infrastructure test
- [x] Push acknowledgment test
- [x] Postgres changes test
- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] More examples

## Porting from TypeScript

This project is being ported from [@supabase/realtime-js](https://github.com/supabase/realtime-js).

Key differences:
- **Callbacks → Traits/Channels**: JavaScript callbacks are replaced with Rust traits and async channels
- **Shared State**: Uses `Arc<RwLock<T>>` for thread-safe shared state
- **Error Handling**: Uses `Result<T, RealtimeError>` instead of exceptions
- **Async/Await**: Native Tokio async/await instead of Promises

## Contributing

This is a starter boilerplate. Contributions are welcome!

## License

MIT
