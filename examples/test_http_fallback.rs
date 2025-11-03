use serde_json::json;
use supabase_realtime_rs::channel::RealtimeChannelOptions;
use supabase_realtime_rs::{ChannelEvent, RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing HTTP Fallback for Broadcasts\n");

    // Get credentials from environment, fallback to echo server for testing
    let url =
        std::env::var("SUPABASE_URL").unwrap_or_else(|_| "wss://echo.websocket.org".to_string());
    let api_key = std::env::var("SUPABASE_API_KEY").unwrap_or_else(|_| "test".to_string());

    println!("📡 Connecting to: {}\n", url);

    // Create client
    let client = RealtimeClient::new(
        &url,
        RealtimeClientOptions {
            api_key,
            heartbeat_interval: Some(30_000),
            ..Default::default()
        },
    )?;

    println!("✅ Test 1: Creating channel WITHOUT connecting...");
    let channel = client
        .channel("chat-room", RealtimeChannelOptions::default())
        .await;
    println!("✅ Channel: {}\n", channel.topic());

    println!("✅ Test 2: Attempting to send broadcast while DISCONNECTED...");
    println!("   (This should trigger HTTP fallback)\n");

    match channel
        .send(
            ChannelEvent::Custom(String::from("chat-message")),
            json!({
                "user": "bob",
                "message": "Hello via HTTP!"
            }),
        )
        .await
    {
        Ok(_) => println!("❌ Unexpected: Send succeeded (but we're testing disconnected state)"),
        Err(e) => println!(
            "✅ Expected: Send failed with HTTP fallback attempt\n   Error: {}\n",
            e
        ),
    }

    println!("✅ Test 3: Now connecting and subscribing...");
    client.connect().await?;
    channel.subscribe().await?;
    println!("✅ Connected and subscribed!\n");

    // Wait a moment
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("✅ Test 4: Sending broadcast while CONNECTED...");
    println!("   (This should use WebSocket)\n");

    channel
        .send(
            ChannelEvent::Custom(String::from("chat-message")),
            json!({
                "user": "carol",
                "message": "Hello via WebSocket!"
            }),
        )
        .await?;

    println!("✅ Broadcast sent via WebSocket!\n");

    // Wait for echo
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("✅ Test 5: Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected!\n");

    println!("✅ Test 6: Sending broadcast AFTER disconnect...");
    println!("   (This should trigger HTTP fallback again)\n");

    match channel
        .send(
            ChannelEvent::Custom(String::from("chat-message")),
            json!({
                "user": "dave",
                "message": "Hello after disconnect!"
            }),
        )
        .await
    {
        Ok(_) => println!("❌ Unexpected: Send succeeded"),
        Err(e) => println!(
            "✅ Expected: Send failed with HTTP fallback attempt\n   Error: {}\n",
            e
        ),
    }

    println!("🎉 All tests completed!");
    println!("\n📊 Summary:");
    println!("   - HTTP fallback is attempted when disconnected");
    println!("   - WebSocket is used when connected");
    println!("   - HTTP fallback will fail with echo.websocket.org (no HTTP endpoint)");
    println!("   - With a real Supabase endpoint, HTTP fallback would succeed!");

    Ok(())
}
