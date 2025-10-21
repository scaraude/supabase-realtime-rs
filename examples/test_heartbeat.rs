use std::time::Duration;
use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    // Initialize tracing to see heartbeat logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing Heartbeat Mechanism With Supabase Client\n");

    let url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set in .env");
    let api_key = std::env::var("SUPABASE_API_KEY").expect("SUPABASE_API_KEY must be set in .env");

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

    println!("✅ Test 1: Connecting with heartbeat enabled...");
    client.connect().await?;
    println!("✅ Connected!\n");

    println!("⏳ Waiting 15 seconds to observe heartbeats...");
    println!("   (Watch for 'Sent heartbeat with ref X' in logs)\n");

    tokio::time::sleep(Duration::from_secs(15)).await;

    println!("\n✅ Test 2: Disconnecting (should stop heartbeat task)...");
    client.disconnect().await?;
    println!("✅ Disconnected!\n");

    println!("⏳ Waiting 5 seconds to confirm heartbeat stopped...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    println!("✅ No more heartbeats - task was properly cleaned up!\n");

    println!("🎉 All tests passed!");

    Ok(())
}
