use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
use supabase_realtime_rs::channel::RealtimeChannelOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing Channel Subscribe/Unsubscribe\n");

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

    println!("✅ Connecting to server...");
    client.connect().await?;
    println!("✅ Connected!\n");

    println!("✅ Creating channel...");
    let channel = client.channel("test-room", RealtimeChannelOptions::default()).await;
    println!("✅ Channel: {}\n", channel.topic());

    println!("✅ Registering listener for 'phx_join' and 'phx_leave'...");
    let mut join_rx = channel.on("phx_join").await;
    let mut leave_rx = channel.on("phx_leave").await;

    // Spawn task to listen for join events
    tokio::spawn(async move {
        while let Some(payload) = join_rx.recv().await {
            println!("📨 Received JOIN event: {:?}", payload);
        }
    });

    // Spawn task to listen for leave events
    tokio::spawn(async move {
        while let Some(payload) = leave_rx.recv().await {
            println!("📨 Received LEAVE event: {:?}", payload);
        }
    });

    println!("✅ Event listeners registered!\n");

    // Test 1: Subscribe
    println!("📤 Test 1: Subscribing to channel...");
    channel.subscribe().await?;
    println!("✅ Subscribed!\n");

    // Wait for echo
    println!("⏳ Waiting 1 second for server echo...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test 2: Unsubscribe
    println!("\n📤 Test 2: Unsubscribing from channel...");
    channel.unsubscribe().await?;
    println!("✅ Unsubscribed!\n");

    // Wait for echo
    println!("⏳ Waiting 1 second for server echo...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test 3: Re-subscribe
    println!("\n📤 Test 3: Re-subscribing to channel...");
    channel.subscribe().await?;
    println!("✅ Re-subscribed!\n");

    // Wait for echo
    println!("⏳ Waiting 1 second for server echo...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("\n✅ Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected!\n");

    println!("🎉 All tests completed!");
    println!("\n📊 Expected output:");
    println!("   - Received JOIN event (from subscribe)");
    println!("   - Received LEAVE event (from unsubscribe)");
    println!("   - Received JOIN event (from re-subscribe)");

    Ok(())
}
