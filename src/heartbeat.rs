use crate::connection::ConnectionManager;
use crate::types::message::RealtimeMessage;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time;

const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(30000);

pub struct HeartbeatManager {
    interval: Duration,
    pending_ref: Arc<RwLock<Option<String>>>,
    connection: Weak<ConnectionManager>,
    ref_counter: Arc<RwLock<u64>>,
}

impl HeartbeatManager {
    pub fn new_with_counter(
        connection: Weak<ConnectionManager>,
        ref_counter: Arc<RwLock<u64>>,
        pending_ref: Arc<RwLock<Option<String>>>,
    ) -> Self {
        Self {
            interval: DEFAULT_HEARTBEAT_INTERVAL,
            pending_ref,
            connection,
            ref_counter,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Spawns the heartbeat task that runs periodically
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = time::interval(self.interval);
            interval_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

            loop {
                interval_timer.tick().await;

                // Check if connection is still valid
                let connection = match self.connection.upgrade() {
                    Some(conn) => conn,
                    None => {
                        // Client dropped, exit heartbeat task
                        break;
                    }
                };

                // Check if connected
                if !connection.is_connected().await {
                    continue;
                }

                // Check for pending heartbeat (timeout detection)
                {
                    let pending = self.pending_ref.read().await;
                    if pending.is_some() {
                        eprintln!("[Heartbeat] Timeout detected, closing connection");
                        let _ = connection.close().await;
                        continue;
                    }
                }

                // Generate new ref
                let new_ref = {
                    let mut counter = self.ref_counter.write().await;
                    *counter += 1;
                    counter.to_string()
                };

                let heartbeat_msg = RealtimeMessage {
                    topic: "phoenix".to_string(),
                    event: "heartbeat".to_string(),
                    payload: serde_json::json!({}),
                    r#ref: Some(new_ref.clone()),
                    join_ref: None,
                };

                match connection.send_message(heartbeat_msg).await {
                    Ok(_) => {
                        let mut pending = self.pending_ref.write().await;
                        *pending = Some(new_ref.clone());
                        tracing::debug!("Sent heartbeat with ref {}", new_ref);
                    }
                    Err(e) => {
                        tracing::error!("[Heartbeat] Failed to send: {}", e);
                    }
                }
            }
        })
    }

    /// Clears the pending heartbeat ref (call this when heartbeat reply received)
    pub async fn clear_pending_ref(&self) {
        let mut pending = self.pending_ref.write().await;
        *pending = None;
    }
}
