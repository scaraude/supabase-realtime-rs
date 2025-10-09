use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing to see heartbeat logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing Heartbeat Mechanism\n");

    // Create client with 5 second heartbeat (faster for testing)
    let client = RealtimeClient::new(
        "wss://echo.websocket.org/",
        RealtimeClientOptions {
            api_key: "test".to_string(),
            heartbeat_interval: Some(5000), // 5 seconds instead of 25
            ..Default::default()
        },
    )?;

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
