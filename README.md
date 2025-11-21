[![CI](https://github.com/Scaraude/supabase-realtime-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Scaraude/supabase-realtime-rs/actions/workflows/ci.yml)

# Supabase Realtime Rust 🦀

A Rust client for [Supabase Realtime](https://supabase.com/docs/guides/realtime) implementing the Phoenix Channels WebSocket protocol.

> **Note**: This is an unofficial, community-maintained client. For official clients, see [supabase-community](https://github.com/supabase-community).

## Quick Start

```rust
use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions, ChannelEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Supabase Realtime
    let client = RealtimeClient::new(
        "wss://your-project.supabase.co/realtime/v1",
        RealtimeClientOptions {
            api_key: "your-anon-key".to_string(),
            ..Default::default()
        },
    )?;
    client.connect().await?;

    // Subscribe to a channel
    let channel = client.channel("room:lobby", Default::default()).await;
    let mut rx = channel.on(ChannelEvent::broadcast("message")).await;
    channel.subscribe().await?;

    // Send and receive messages
    channel.send(
        ChannelEvent::broadcast("message"),
        serde_json::json!({"text": "Hello!"})
    ).await?;

    // Listen for messages
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            println!("Received: {:?}", msg);
        }
    });

    Ok(())
}
```

## Features

### Core Functionality
- ✅ **WebSocket connection** with automatic reconnection and exponential backoff
- ✅ **Channel subscriptions** for pub/sub messaging
- ✅ **Broadcast messaging** with HTTP fallback when disconnected
- ✅ **Postgres changes** - Subscribe to database INSERT/UPDATE/DELETE events
- ✅ **Presence tracking** - Track online users with custom metadata
- ✅ **Push messages** with acknowledgments (ok/error/timeout callbacks)
- ✅ **Type-safe** error handling throughout

### Technical Features
- Async/await with [Tokio](https://tokio.rs)
- Thread-safe shared state with `Arc<RwLock<T>>`
- Event routing via mpsc channels
- TLS support with `native-tls`
- Heartbeat mechanism with timeout detection

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
supabase-realtime-rs = { git = "https://github.com/Scaraude/realtime-rust" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## Usage Examples

### Database Changes (Postgres)

```rust
use supabase_realtime_rs::{PostgresChangesFilter, PostgresChangeEvent};

let channel = client.channel("db-changes", Default::default()).await;

// Listen for all changes to the "todos" table
let mut rx = channel.on_postgres_changes(
    PostgresChangesFilter::new(PostgresChangeEvent::All, "public")
        .table("todos")
).await;

channel.subscribe().await?;

tokio::spawn(async move {
    while let Some(change) = rx.recv().await {
        println!("Database change: {:?}", change);
    }
});
```

**Note**: Requires Row Level Security (RLS) policies with SELECT permissions on the table.

### Presence Tracking

```rust
use serde_json::json;

let channel = client.channel("room:lobby", Default::default()).await;
channel.subscribe().await?;

// Track your presence
channel.track(json!({
    "user": "Alice",
    "status": "online",
    "cursor_x": 100,
    "cursor_y": 200
})).await?;

// Get all present users
let users = channel.presence_list().await;
println!("Online users: {:?}", users);

// Stop tracking
channel.untrack().await?;
```

### Push Messages with Callbacks

```rust
channel.push("custom_event", serde_json::json!({"data": "value"}))
    .receive("ok", |payload| {
        println!("Success: {:?}", payload);
    })
    .receive("error", |payload| {
        eprintln!("Error: {:?}", payload);
    })
    .receive("timeout", |_| {
        eprintln!("Request timed out");
    })
    .send()
    .await?;
```

## Examples

The [`examples/`](examples/) directory contains working code for all features:

```bash
# Setup (first time only)
cp .env.example .env
# Edit .env with your Supabase credentials

# Run examples
cargo run --example test_connection       # Basic WebSocket connection
cargo run --example test_channel          # Channel subscriptions
cargo run --example test_send             # Broadcasting messages
cargo run --example test_postgres_changes # Database event streaming
cargo run --example test_presence         # Presence tracking
cargo run --example test_push             # Push with acknowledgments
```

## Production Readiness

### Status: Beta (v0.1.0)

**Production-ready features:**
- ✅ All core Phoenix Channels protocol features
- ✅ Robust reconnection and error handling
- ✅ Comprehensive examples and documentation
- ✅ CI/CD with automated testing

**Known limitations:**
- ⚠️ No automatic JWT token refresh (requires manual reconnect when token expires)
- ⚠️ No postgres type transformers (values received as raw strings, manual parsing required)
- ⚠️ Limited to Phoenix Channels features (no Supabase Auth integration yet)

See [Cargo.toml](Cargo.toml) for dependency versions.

## Why Rust?

**Performance**: Zero-cost abstractions, compiled binary, native async/await

**Safety**: Memory-safe and thread-safe by default, preventing entire classes of bugs

**Use Cases**:
- High-performance servers and microservices
- CLI tools and system utilities
- WebAssembly applications
- Embedded systems
- Anywhere JavaScript/Node.js is too slow or too heavy

## Migrating from TypeScript

Key differences from [@supabase/realtime-js](https://github.com/supabase/realtime-js):

| Concept | JavaScript | Rust |
|---------|-----------|------|
| **Callbacks** | `channel.on('event', (payload) => {})` | `let mut rx = channel.on(event).await` |
| **Error Handling** | `try/catch` | `Result<T, E>` with `?` operator |
| **Async** | `async/await` (Promise-based) | `async/await` (Future-based with Tokio) |
| **Shared State** | Direct mutation | `Arc<RwLock<T>>` for thread safety |
| **Event Listening** | Single callback per event | mpsc channels (multiple consumers possible) |

**Example comparison:**

```javascript
// JavaScript
const channel = client.channel('room:lobby')
channel.on('broadcast', { event: 'message' }, (payload) => {
  console.log(payload)
})
await channel.subscribe()
```

```rust
// Rust
let channel = client.channel("room:lobby", Default::default()).await;
let mut rx = channel.on(ChannelEvent::broadcast("message")).await;
channel.subscribe().await?;

tokio::spawn(async move {
    while let Some(payload) = rx.recv().await {
        println!("{:?}", payload);
    }
});
```

## Architecture

Built on idiomatic Rust patterns:

- **Connection Management** - WebSocket lifecycle with automatic reconnection
- **Message Routing** - Routes incoming messages to appropriate channel handlers
- **Channel System** - Subscribe to topics, filter events, manage presence
- **Infrastructure** - Heartbeat, HTTP fallback, background task management

Uses [Phoenix Channels protocol](https://hexdocs.pm/phoenix/channels.html) for compatibility with Supabase Realtime.

## Development

### Build & Test

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy --all-features -- -D warnings

# Generate documentation
cargo doc --open
```

### Contributing

Contributions are welcome! Here's how to get started:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Make** your changes
4. **Test** your changes (`cargo test && cargo clippy`)
5. **Commit** (`git commit -m 'Add amazing feature'`)
6. **Push** (`git push origin feature/amazing-feature`)
7. **Open** a Pull Request

**Code style:**
- Follow Rust naming conventions (snake_case, PascalCase)
- Add rustdoc comments (`///`) to public APIs
- Include examples in documentation
- Use `Result<T, E>` for error handling, avoid `unwrap()`

**Need help?** Open an issue or discussion!

## License

MIT License - see [LICENSE](LICENSE) for details

## Acknowledgments

- Built on [Tokio](https://tokio.rs) async runtime
- Implements [Phoenix Channels](https://hexdocs.pm/phoenix/channels.html) protocol
- Ported from [@supabase/realtime-js](https://github.com/supabase/realtime-js)

---

**Made with 🦀 by the Rust community**
