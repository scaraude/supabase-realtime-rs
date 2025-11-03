//! Advanced Presence Example
//!
//! This example demonstrates advanced presence tracking features:
//! - Event listeners for presence_state and presence_diff
//! - Real-time presence updates
//! - Tracking and updating presence metadata
//! - Inspecting presence state at different stages
//!
//! ## Key Concepts:
//!
//! **One Presence Per Connection**: Each WebSocket connection can only have one presence
//! entry at a time. When you call `track()` multiple times on the same connection,
//! you're *updating* the presence metadata, not adding multiple users.
//!
//! **presence_key**: Identifies unique users. All presence tracking for this connection
//! is grouped under this key (e.g., "user_id").
//!
//! **Events vs State**:
//! - `presence_state`: Full snapshot of all present users (sent on initial sync)
//! - `presence_diff`: Incremental changes (joins/leaves)
//! - `presence_list()`: Your local view of who's currently present

use supabase_realtime_rs::{
    ChannelEvent, RealtimeChannelOptions, RealtimeClient, RealtimeClientOptions,
};

async fn display_presence_list(channel: &std::sync::Arc<supabase_realtime_rs::RealtimeChannel>) {
    println!("📋 Checking presence state...");
    let presence = channel.presence_list().await;
    println!("   Users present: {} entries", presence.len());
    for (key, metas) in &presence {
        println!("   - {}: {:?}", key, metas);
    }
    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Use INFO level to reduce noise in output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🦀 Advanced Presence Tracking Example\n");
    println!("This example shows how to:");
    println!("  1. Listen to presence events (state & diff)");
    println!("  2. Track and update presence metadata");
    println!("  3. Inspect presence state changes in real-time\n");

    // Get credentials from environment
    let url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set in .env");
    let api_key = std::env::var("SUPABASE_API_KEY").expect("SUPABASE_API_KEY must be set in .env");

    println!("📡 Connecting to: {}\n", url);

    // Build the client
    let client = RealtimeClient::new(
        &url,
        RealtimeClientOptions {
            api_key,
            ..Default::default()
        },
    )?;

    // Connect to server
    println!("✅ Step 1: Connecting to server...");
    client.connect().await?;
    println!("   Connected!\n");

    // Create channel with presence enabled
    println!("✅ Step 2: Creating channel with presence enabled...");
    let channel = client
        .channel(
            "room:advanced-lobby",
            RealtimeChannelOptions {
                presence_key: Some("user_id".to_string()),
                ..Default::default()
            },
        )
        .await;
    println!("   Channel created: {}\n", channel.topic());

    // Set up event listeners BEFORE subscribing
    // This ensures we don't miss the initial presence_state event
    println!("✅ Step 3: Setting up event listeners...");
    let mut presence_state_rx = channel.on(ChannelEvent::PresenceState).await;
    let mut presence_diff_rx = channel.on(ChannelEvent::PresenceDiff).await;

    // Spawn background tasks to handle presence events
    tokio::spawn(async move {
        while let Some(payload) = presence_state_rx.recv().await {
            println!("📥 [presence_state] Full presence snapshot received:");
            println!("   {}\n", serde_json::to_string_pretty(&payload).unwrap());
        }
    });
    tokio::spawn(async move {
        while let Some(payload) = presence_diff_rx.recv().await {
            println!("📥 [presence_diff] Presence changes detected:");
            println!("   {}\n", serde_json::to_string_pretty(&payload).unwrap());
        }
    });
    println!("   Listeners ready!\n");

    println!("✅ Step 4: Subscribing to channel...");
    channel.subscribe().await?;
    println!("   Subscribed!\n");

    display_presence_list(&channel).await;

    // Track presence as "Ludo"
    // This is the FIRST track call - creates initial presence for this connection
    println!("✅ Step 5: Tracking presence as 'Ludo'...");
    channel
        .track(serde_json::json!({
         "user": "Ludo",
         "status": "online"
        }))
        .await?;
    println!("   Presence tracked!\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    display_presence_list(&channel).await;

    // Track presence as "Nico"
    // IMPORTANT: This UPDATES the existing presence (doesn't add a second user!)
    // Same connection = same presence entry, just different metadata
    println!("✅ Step 6: Updating presence to 'Nico'...");
    println!("   (Note: This replaces 'Ludo' because it's the same connection)\n");
    channel
        .track(serde_json::json!({
            "user": "Nico",
            "status": "away"
        }))
        .await?;
    println!("   Presence updated!\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    display_presence_list(&channel).await;

    // Untrack presence - removes this connection from presence
    println!("✅ Step 7: Untracking presence...");
    channel.untrack().await?;
    println!("   Presence untracked!\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    display_presence_list(&channel).await;

    println!("✅ All steps completed!\n");

    // Cleanup
    client.disconnect().await?;

    Ok(())
}
