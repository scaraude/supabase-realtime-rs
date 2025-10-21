use supabase_realtime_rs::channel::RealtimeChannelOptions;
use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing Channel Subscription\n");

    // Get credentials from environment, fallback to echo server for testing
    let url = std::env::var("SUPABASE_URL").unwrap_or_else(|_| "wss://echo.websocket.org".to_string());
    let api_key = std::env::var("SUPABASE_API_KEY").unwrap_or_else(|_| "test".to_string());

    println!("📡 Connecting to: {}\n", url);

    // Create client
    let client = RealtimeClient::new(
        &url,
        RealtimeClientOptions {
            api_key,
            heartbeat_interval: Some(30_000), // 30 seconds
            ..Default::default()
        },
    )?
    .build();

    println!("✅ Test 1: Creating a channel...");
    let channel = client
        .channel("test-room", RealtimeChannelOptions::default())
        .await;
    println!("✅ Channel created: {}\n", channel.topic());

    println!("✅ Test 2: Registering event listener for 'phx_join'...");
    let mut rx = channel.on("phx_join").await;
    println!("✅ Event listener registered!\n");

    println!("✅ Test 3: Connecting to server...");
    client.connect().await?;
    println!("✅ Connected!\n");

    println!("✅ Test 4: Subscribing to channel...");
    channel.subscribe().await?;
    println!("✅ Subscribed!\n");

    println!("✅ Test 5: Listening for 'phx_join' events...");
    println!("   (Echo server will echo back our subscription)\n");

    // Spawn a task to listen for events
    let event_listener = tokio::spawn(async move {
        let mut count = 0;
        while let Some(payload) = rx.recv().await {
            count += 1;
            println!("📨 Received event #{}: {:?}", count, payload);
        }
    });

    // Wait a bit for the join message to be echoed back
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("\n✅ Test 5: Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected!\n");

    // Give event listener time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    event_listener.abort();

    println!("🎉 All tests completed!");

    Ok(())
}
