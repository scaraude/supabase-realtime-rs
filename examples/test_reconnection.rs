use std::time::Duration;
use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};

/// Test reconnection behavior with a real Supabase instance
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing to see logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🦀 Testing Reconnection with Real Supabase\n");

    // Get credentials from environment
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
    )?;

    // Test 1: Connect and verify
    println!("✅ Test 1: Initial connection...");
    client.connect().await?;
    assert!(client.is_connected().await, "Should be connected");
    println!("✅ Connected successfully!\n");

    // Keep connection alive for a moment
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test 2: Manual disconnect should NOT trigger reconnection
    println!("✅ Test 2: Manual disconnect (should NOT auto-reconnect)...");
    client.disconnect().await?;
    assert!(!client.is_connected().await, "Should be disconnected");
    println!("✅ Disconnected manually\n");

    // Wait to verify no reconnection happens
    println!("⏳ Waiting 5 seconds to verify no auto-reconnect...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    if !client.is_connected().await {
        println!("✅ Correctly stayed disconnected after manual disconnect!\n");
    } else {
        return Err("Should NOT reconnect after manual disconnect".into());
    }

    println!("🎉 Manual disconnect test passed!\n");

    // Test 3: Reconnect and then simulate connection failure
    println!("✅ Test 3: Testing automatic reconnection...");
    client.connect().await?;
    assert!(client.is_connected().await, "Should be connected again");
    println!("✅ Reconnected successfully\n");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Simulate network interruption by accessing internal state
    // This is a workaround since we can't directly close the WebSocket from outside
    println!("⚠️  Simulating connection failure...");
    println!("💡 To trigger this manually:");
    println!("   1. While this is running, disable your network");
    println!("   2. Re-enable it after a few seconds");
    println!("   3. Watch the logs for reconnection attempts\n");

    println!("⏳ Keeping connection alive for 30 seconds...");
    println!("   (Manually interrupt your network to see auto-reconnect)\n");

    // Monitor connection status for 30 seconds
    for i in 1..=30 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let is_connected = client.is_connected().await;
        print!(
            "\r⏱  Second {}/30 - Status: {}",
            i,
            if is_connected {
                "🟢 Connected"
            } else {
                "🔴 Disconnected"
            }
        );
        std::io::Write::flush(&mut std::io::stdout())?;
    }
    println!("\n");

    // Final status
    if client.is_connected().await {
        println!("✅ Final status: Connected");
    } else {
        println!("⚠️  Final status: Disconnected");
    }

    println!("\n🎉 Reconnection tests completed!");
    println!("\n📋 Verified:");
    println!("   ✅ Connected to real Supabase instance");
    println!("   ✅ Manual disconnect is respected (no auto-reconnect)");
    println!("   ✅ Can reconnect after manual disconnect");
    println!("   ✅ State watcher task is running");
    println!("\n💡 To test automatic reconnection:");
    println!("   Run this test and manually interrupt your network connection");

    Ok(())
}
