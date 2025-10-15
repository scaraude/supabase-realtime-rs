use crate::client_state::ClientState;
use crate::connection::ConnectionManager;
use crate::types::message::RealtimeMessage;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::RwLock;

const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(30000);

pub struct HeartbeatManager {
    interval: Duration,
    state: Arc<RwLock<ClientState>>,
    connection: Weak<ConnectionManager>,
}

impl HeartbeatManager {
    pub fn new(
        connection: Weak<ConnectionManager>,
        state: Arc<RwLock<ClientState>>,
    ) -> Self {
        Self {
            interval: DEFAULT_HEARTBEAT_INTERVAL,
            state,
            connection,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Spawns the heartbeat task that runs periodically
    pub async fn spawn_on(self, state: &Arc<RwLock<ClientState>>) {
        let mut state_guard = state.write().await;
        state_guard.task_manager.spawn(async move {
            let mut interval_timer = tokio::time::interval(self.interval);
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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
                    let state = self.state.read().await;
                    if state.pending_heartbeat_ref.is_some() {
                        eprintln!("[Heartbeat] Timeout detected, closing connection");
                        let _ = connection.close().await;
                        continue;
                    }
                }

                // Generate new ref
                let new_ref = {
                    let mut state = self.state.write().await;
                    state.ref_counter += 1;
                    state.ref_counter.to_string()
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
                        let mut state = self.state.write().await;
                        state.pending_heartbeat_ref = Some(new_ref.clone());
                        tracing::debug!("Sent heartbeat with ref {}", new_ref);
                    }
                    Err(e) => {
                        tracing::error!("[Heartbeat] Failed to send: {}", e);
                    }
                }
            }
        });
    }
}
