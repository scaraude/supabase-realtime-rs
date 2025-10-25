use supabase_realtime_rs::{
    PostgresChangeEvent, PostgresChangesFilter, RealtimeChannelOptions, RealtimeClient,
    RealtimeClientOptions,
};

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
            api_key: api_key.clone(),
            ..Default::default()
        },
    )?
    .build();

    println!("Connecting to Supabase Realtime...");
    client.connect().await?;
    println!("Connected!");

    // Create a channel
    let channel = client
        .channel("schema-db-changes", RealtimeChannelOptions::default())
        .await;

    println!("Setting up postgres_changes listeners...");

    // Listen to INSERT events on the 'users' table
    let filter_insert =
        PostgresChangesFilter::new(PostgresChangeEvent::Insert, "public").table("users");

    let mut insert_rx = channel.on_postgres_changes(filter_insert).await;

    // Listen to UPDATE events on the 'users' table
    let filter_update =
        PostgresChangesFilter::new(PostgresChangeEvent::Update, "public").table("users");

    let mut update_rx = channel.on_postgres_changes(filter_update).await;

    // Listen to DELETE events on the 'users' table
    let filter_delete =
        PostgresChangesFilter::new(PostgresChangeEvent::Delete, "public").table("users");

    let mut delete_rx = channel.on_postgres_changes(filter_delete).await;

    // Listen to all events on the 'posts' table
    let filter_posts_all =
        PostgresChangesFilter::new(PostgresChangeEvent::All, "public").table("posts");

    let mut posts_rx = channel.on_postgres_changes(filter_posts_all).await;

    // Spawn tasks to handle events
    tokio::spawn(async move {
        println!("Listening for INSERT events on users table...");
        while let Some(payload) = insert_rx.recv().await {
            println!(
                "[{}] 🆕 INSERT on users: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                serde_json::to_string_pretty(&payload).unwrap()
            );
        }
    });

    tokio::spawn(async move {
        println!("Listening for UPDATE events on users table...");
        while let Some(payload) = update_rx.recv().await {
            println!(
                "[{}] 📝 UPDATE on users: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                serde_json::to_string_pretty(&payload).unwrap()
            );
        }
    });

    tokio::spawn(async move {
        println!("Listening for DELETE events on users table...");
        while let Some(payload) = delete_rx.recv().await {
            println!(
                "[{}] 🗑️  DELETE on users: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                serde_json::to_string_pretty(&payload).unwrap()
            );
        }
    });

    tokio::spawn(async move {
        println!("Listening for ALL events on posts table...");
        while let Some(payload) = posts_rx.recv().await {
            println!(
                "[{}] 📬 Event on posts: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                serde_json::to_string_pretty(&payload).unwrap()
            );
        }
    });

    // Subscribe to the channel
    println!("Subscribing to channel...");
    channel.subscribe().await?;
    println!("Subscribed! Waiting for database changes...");
    println!("\nTo test this example:");
    println!("1. Make sure you have a Supabase project with Realtime enabled");
    println!("2. Set SUPABASE_URL and SUPABASE_API_KEY environment variables");
    println!("3. Create tables 'users' and 'posts' in the 'public' schema");
    println!("4. Perform INSERT/UPDATE/DELETE operations on these tables");
    println!("5. Watch the events appear here!\n");

    // Keep connection alive
    tokio::signal::ctrl_c().await?;

    // Cleanup
    println!("\nUnsubscribing...");
    channel.unsubscribe().await?;
    println!("Disconnecting...");
    client.disconnect().await?;
    println!("Done!");

    Ok(())
}
