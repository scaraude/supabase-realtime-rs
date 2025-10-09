use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create client
    let client = RealtimeClient::new(
        "wss://your-project.supabase.co/realtime/v1",
        RealtimeClientOptions {
            api_key: "your-anon-key".to_string(),
            ..Default::default()
        },
    )?;

    // Connect
    println!("Connecting to Supabase Realtime...");
    client.connect().await?;
    println!("Connected!");

    // Keep connection alive
    tokio::signal::ctrl_c().await?;

    // Disconnect
    println!("Disconnecting...");
    client.disconnect().await?;
    println!("Disconnected!");

    Ok(())
}
