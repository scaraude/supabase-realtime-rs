use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
use supabase_realtime_rs::channel::RealtimeChannelOptions;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing Channel Send (Broadcast)\n");

    // Create client
    let client = RealtimeClient::new(
        "wss://echo.websocket.org",
        RealtimeClientOptions {
            api_key: "test".to_string(),
            heartbeat_interval: Some(30_000),
            ..Default::default()
        },
    )?
    .build();

    println!("✅ Test 1: Connecting to server...");
    client.connect().await?;
    println!("✅ Connected!\n");

    println!("✅ Test 2: Creating channel...");
    let channel = client.channel("chat-room", RealtimeChannelOptions::default()).await;
    println!("✅ Channel: {}\n", channel.topic());

    println!("✅ Test 3: Subscribing to channel...");
    channel.subscribe().await?;
    println!("✅ Subscribed!\n");

    // Listen for broadcast events
    println!("✅ Test 4: Registering listener for broadcast events...");
    let mut broadcast_rx = channel.on("broadcast:chat-message").await;

    tokio::spawn(async move {
        while let Some(payload) = broadcast_rx.recv().await {
            println!("📨 Received broadcast: {:?}", payload);
        }
    });

    // Wait a moment for subscription to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("✅ Test 5: Sending broadcast message via WebSocket...");
    channel.send("chat-message", json!({
        "user": "alice",
        "message": "Hello from Rust!"
    })).await?;
    println!("✅ Broadcast sent!\n");

    // Wait for echo
    println!("⏳ Waiting 2 seconds for echo...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("\n✅ Test 6: Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected!\n");

    println!("🎉 All tests completed!");
    println!("\n📊 Note: Echo server will echo back the broadcast message");

    Ok(())
}
