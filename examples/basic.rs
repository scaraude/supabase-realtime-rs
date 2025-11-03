use supabase_realtime_rs::{RealtimeClient, RealtimeClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

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
