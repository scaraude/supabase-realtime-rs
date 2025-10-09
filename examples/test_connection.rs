use realtime_rust::{RealtimeClient, RealtimeClientOptions};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing to see logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing WebSocket Connection\n");

    // Create client (using echo.websocket.org - a public test server)
    let client = RealtimeClient::new(
        "wss://echo.websocket.org/",
        RealtimeClientOptions {
            api_key: "test".to_string(), // Echo server doesn't check this
            ..Default::default()
        },
    )?;

    // Test 1: Connect
    println!("✅ Test 1: Connecting...");
    client.connect().await?;
    println!("✅ Connected successfully!\n");

    // Test 2: Check connection state
    println!("✅ Test 2: Checking connection state...");
    assert!(client.is_connected().await, "Should be connected");
    println!("✅ Connection state is correct!\n");

    // Keep connection alive for a bit
    println!("⏳ Keeping connection alive for 2 seconds...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test 3: Disconnect
    println!("✅ Test 3: Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected successfully!\n");

    // Test 4: Check disconnected state
    println!("✅ Test 4: Checking disconnected state...");
    assert!(!client.is_connected().await, "Should be disconnected");
    println!("✅ Disconnection state is correct!\n");

    println!("🎉 All tests passed!");

    Ok(())
}
