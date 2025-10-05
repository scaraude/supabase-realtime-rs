# Realtime Rust 🦀

A Rust client for [Supabase Realtime](https://supabase.com/docs/guides/realtime) - Phoenix Channels WebSocket protocol implementation.

> ⚠️ **Work in Progress** - This is a starter boilerplate. Core WebSocket functionality is not yet implemented.

## Features (Planned)

- ✅ Type-safe error handling with `thiserror`
- ✅ Async/await with Tokio
- ✅ WebSocket support with `tokio-tungstenite`
- ⏳ Channel subscriptions
- ⏳ Real-time Postgres changes
- ⏳ Presence tracking
- ⏳ Broadcast messages
- ⏳ Automatic reconnection with exponential backoff
- ⏳ Heartbeat mechanism

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
realtime-rust = { path = "../realtime-rust" }
```

## Usage

```rust
use realtime_rust::{RealtimeClient, RealtimeClientOptions};

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

Run the basic example:

```bash
cargo run --example basic
```

## Project Structure

```
src/
├── lib.rs           # Public API exports
├── client.rs        # RealtimeClient - WebSocket connection management
├── channel.rs       # RealtimeChannel - Channel subscriptions
├── presence.rs      # RealtimePresence - User presence tracking
├── push.rs          # Push - Message sending with callbacks
├── timer.rs         # Timer - Reconnection logic with backoff
├── types/           # Type definitions
│   ├── constants.rs # Protocol constants
│   ├── error.rs     # Error types
│   └── message.rs   # Message types
└── websocket/       # WebSocket abstraction
    └── factory.rs   # WebSocket factory
```

## Development Roadmap

### Phase 1: Core Infrastructure ✅
- [x] Project setup
- [x] Type definitions
- [x] Error handling
- [x] Basic client structure

### Phase 2: WebSocket Implementation (Next)
- [ ] WebSocket connection
- [ ] Message serialization/deserialization
- [ ] Connection state management
- [ ] Heartbeat mechanism

### Phase 3: Channels
- [ ] Channel join/leave
- [ ] Event listeners
- [ ] Push/receive messages
- [ ] HTTP fallback for broadcasts

### Phase 4: Advanced Features
- [ ] Presence tracking
- [ ] Postgres changes subscription
- [ ] Reconnection logic
- [ ] Access token refresh

### Phase 5: Testing & Polish
- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] Examples

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
