//! Multi-User Presence Example
//!
//! This example demonstrates TRUE multi-user presence tracking by:
//! - Creating multiple WebSocket connections (simulating different users)
//! - Tracking each user's presence independently
//! - Observing how presence state shows ALL users simultaneously
//! - Demonstrating real-time presence synchronization
//!
//! ## Key Difference from Advanced Example:
//!
//! In the advanced example, we had ONE connection updating its presence.
//! Here, we have MULTIPLE connections, each with their own presence entry.
//! This simulates a real-world scenario where multiple users are in the same room.

use serde_json::json;
use std::sync::Arc;
use supabase_realtime_rs::{
    ChannelEvent, RealtimeChannel, RealtimeChannelOptions, RealtimeClient, RealtimeClientOptions,
};
use tokio::time::{Duration, sleep};

/// Represents a simulated user with their own connection
struct User {
    name: String,
    client: Arc<RealtimeClient>,
    channel: Arc<RealtimeChannel>,
}

impl User {
    /// Create and connect a new user to the room
    async fn new(
        name: String,
        room: &str,
        url: &str,
        api_key: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("👤 [{}] Connecting...", name);

        // Each user gets their own client and WebSocket connection
        let client = RealtimeClient::new(
            url,
            RealtimeClientOptions {
                api_key,
                ..Default::default()
            },
        )?
        .build();

        client.connect().await?;

        // Join the same room channel
        let channel = client
            .channel(
                room,
                RealtimeChannelOptions {
                    presence_key: Some("user_id".to_string()),
                    ..Default::default()
                },
            )
            .await;

        // Set up presence event listener
        let user_name = name.clone();
        let mut presence_diff_rx = channel.on(ChannelEvent::PresenceDiff).await;
        tokio::spawn(async move {
            while let Some(payload) = presence_diff_rx.recv().await {
                println!(
                    "📥 [{}] Presence changed: {}",
                    user_name,
                    serde_json::to_string_pretty(&payload).unwrap()
                );
            }
        });

        // Subscribe to the channel
        channel.subscribe().await?;

        println!("✅ [{}] Connected and subscribed!", name);

        Ok(Self {
            name,
            client: Arc::new(client),
            channel,
        })
    }

    /// Track this user's presence in the room
    async fn track_presence(&self, status: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "📍 [{}] Tracking presence with status: {}",
            self.name, status
        );
        self.channel
            .track(json!({
                "user": self.name,
                "status": status,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
            .await?;
        Ok(())
    }

    /// Update this user's presence metadata
    async fn update_status(&self, status: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 [{}] Updating status to: {}", self.name, status);
        self.track_presence(status).await
    }

    /// Check and display the current presence list
    async fn check_presence(&self) {
        let presence = self.channel.presence_list().await;

        // Count total users across all presence keys
        let total_users: usize = presence.iter().map(|(_, metas)| metas.len()).sum();

        println!(
            "👥 [{}] sees {} user{} in the room:",
            self.name,
            total_users,
            if total_users == 1 { "" } else { "s" }
        );
        for (_key, metas) in &presence {
            for meta in metas {
                if let Some(user) = meta.data.get("user") {
                    if let Some(status) = meta.data.get("status") {
                        println!("   - {}: {}", user, status);
                    }
                }
            }
        }
        println!();
    }

    /// Untrack presence and disconnect
    async fn leave(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("👋 [{}] Leaving room...", self.name);
        self.channel.untrack().await?;
        self.client.disconnect().await?;
        println!("✅ [{}] Disconnected", self.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN) // Minimal logging for clarity
        .init();

    println!("🦀 Multi-User Presence Example\n");
    println!("Simulating 3 users joining the same room...\n");

    // Get credentials
    let url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let api_key = std::env::var("SUPABASE_API_KEY").expect("SUPABASE_API_KEY must be set");
    let room = "room:multiuser-demo";

    // === PHASE 1: Three users join ===
    println!("=== PHASE 1: Users Joining ===\n");

    let alice = User::new("Alice".to_string(), room, &url, api_key.clone()).await?;
    sleep(Duration::from_millis(500)).await;

    let bob = User::new("Bob".to_string(), room, &url, api_key.clone()).await?;
    sleep(Duration::from_millis(500)).await;

    let charlie = User::new("Charlie".to_string(), room, &url, api_key.clone()).await?;
    sleep(Duration::from_millis(500)).await;

    println!("\n=== PHASE 2: Users Track Presence ===\n");

    // All users track their presence
    alice.track_presence("online").await?;
    sleep(Duration::from_secs(1)).await;

    bob.track_presence("online").await?;
    sleep(Duration::from_secs(1)).await;

    charlie.track_presence("away").await?;
    sleep(Duration::from_secs(2)).await;

    println!("\n=== PHASE 3: Check Presence (All Users See Everyone) ===\n");

    // Each user checks who's in the room
    alice.check_presence().await;
    bob.check_presence().await;
    charlie.check_presence().await;

    println!("\n=== PHASE 4: Users Update Status ===\n");

    // Alice changes status
    alice.update_status("busy").await?;
    sleep(Duration::from_secs(2)).await;

    // Bob changes status
    bob.update_status("away").await?;
    sleep(Duration::from_secs(2)).await;

    println!("\n=== PHASE 5: Check Presence Again ===\n");

    alice.check_presence().await;
    bob.check_presence().await;
    charlie.check_presence().await;

    println!("\n=== PHASE 6: Users Leave One by One ===\n");

    // Charlie leaves
    charlie.leave().await?;
    sleep(Duration::from_secs(2)).await;

    alice.check_presence().await;
    bob.check_presence().await;

    // Bob leaves
    bob.leave().await?;
    sleep(Duration::from_secs(2)).await;

    alice.check_presence().await;

    // Alice leaves
    alice.leave().await?;

    println!("\n🎉 Multi-user presence demo completed!\n");

    Ok(())
}
