use std::time::Duration;
use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing to see logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing WebSocket Connection\n");

    // Get credentials from environment, fallback to echo server for testing
    let url = std::env::var("SUPABASE_URL").unwrap_or_else(|_| "wss://echo.websocket.org/".to_string());
    let api_key = std::env::var("SUPABASE_API_KEY").unwrap_or_else(|_| "test".to_string());

    println!("📡 Connecting to: {}\n", url);

    // Create client
    let client = RealtimeClient::new(
        &url,
        RealtimeClientOptions {
            api_key,
            ..Default::default()
        },
    )?
    .build();

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
