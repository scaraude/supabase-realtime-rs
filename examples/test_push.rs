use supabase_realtime_rs::{
    ChannelEvent, RealtimeChannelOptions, RealtimeClient, RealtimeClientOptions, SystemEvent,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    println!("🦀 Testing Push\n");

    // Get credentials from environment, fallback to echo server for testing
    let url = std::env::var("SUPABASE_URL").unwrap_or_else(|_| "wss://echo.websocket.org".to_string());
    let api_key = std::env::var("SUPABASE_API_KEY").unwrap_or_else(|_| "test".to_string());

    println!("📡 Connecting to: {}\n", url);

    let client = RealtimeClient::new(
        &url,
        RealtimeClientOptions {
            api_key,
            heartbeat_interval: Some(30_000),
            ..Default::default()
        },
    )?
    .build();

    client.connect().await?;
    println!("✅ Connected!\n");

    let channel = client
        .channel("test-push", RealtimeChannelOptions::default())
        .await;
    println!("✅ Created channel: {}\n", channel.topic());

    channel.subscribe().await?;
    println!("✅ Subscribed to channel: {}\n", channel.topic());

    channel
        .push(
            ChannelEvent::System(SystemEvent::Reply),
            serde_json::json!({"message": "Hello, Realtime!", "status": "ok"}),
        )
        .receive("ok", |_| {
            println!("📨 Push acknowledged with 'ok' response")
        })
        .receive("error", |_| {
            println!("❌ Push acknowledged with 'error' response")
        })
        .receive("timeout", |_| println!("⏰ Push timed out"))
        .send()
        .await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    println!("\n✅ Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected!\n");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}
