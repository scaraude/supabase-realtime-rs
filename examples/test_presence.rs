use serde_json::json;
use supabase_realtime_rs::{RealtimeChannelOptions, RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing Presence Tracking\n");

    // Get credentials from environment
    let url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set in .env");
    let api_key = std::env::var("SUPABASE_API_KEY").expect("SUPABASE_API_KEY must be set in .env");

    println!("📡 Connecting to: {}\n", url);

    let client = RealtimeClient::new(
        &url,
        RealtimeClientOptions {
            api_key,
            ..Default::default()
        },
    )?
    .build();

    println!("✅ Test 1: Connecting to server...");
    client.connect().await?;
    println!("✅ Connected!\n");

    println!("✅ Test 2: Creating channel with presence enabled...");
    let channel = client
        .channel(
            "room:lobby",
            RealtimeChannelOptions {
                presence_key: Some("user_id".to_string()),
                ..Default::default()
            },
        )
        .await;
    println!("✅ Channel created: {}\n", channel.topic());

    println!("✅ Test 3: Subscribing to channel...");
    channel.subscribe().await?;
    println!("✅ Subscribed!\n");

    println!("✅ Test 4: Tracking presence...");
    channel
        .track(json!({
            "status": "online",
            "user": "Alice"
        }))
        .await?;
    println!("✅ Presence tracked!\n");

    // Wait for presence sync
    println!("⏳ Waiting for presence sync...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("✅ Test 5: Getting presence list...");
    let presence = channel.presence_list().await;
    println!("📋 Presence list: {:?}\n", presence);

    println!("✅ Test 6: Untracking presence...");
    channel.untrack().await?;
    println!("✅ Presence untracked!\n");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("✅ Test 7: Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected!\n");

    println!("🎉 All tests completed!");

    Ok(())
}
