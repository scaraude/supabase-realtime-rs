use super::connection::ConnectionState;
use crate::channel::RealtimeChannel;
use crate::infrastructure::TaskManager;
use std::sync::Arc;
use tokio::sync::watch;

/// Consolidated mutable state for RealtimeClient
/// Using a single struct reduces lock contention
pub struct ClientState {
    /// Current ref counter for message IDs
    pub ref_counter: u64,

    /// Pending heartbeat ref (if any)
    pub pending_heartbeat_ref: Option<String>,

    /// All channels managed by this client
    pub channels: Vec<Arc<RealtimeChannel>>,

    /// Background task manager
    pub task_manager: TaskManager,

    /// Whether the disconnect was manual (prevents auto-reconnect)
    pub was_manual_disconnect: bool,

    /// Sender for state change notifications
    pub state_change_tx: Option<watch::Sender<(ConnectionState, bool)>>,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            ref_counter: 0,
            pending_heartbeat_ref: None,
            channels: Vec::new(),
            task_manager: TaskManager::new(),
            was_manual_disconnect: false,
            state_change_tx: None,
        }
    }

    /// Generate next message reference
    pub fn make_ref(&mut self) -> String {
        self.ref_counter += 1;
        self.ref_counter.to_string()
    }

    /// Notify state change watchers
    pub fn notify_state_change(&self, state: ConnectionState, manual: bool) {
        if let Some(tx) = &self.state_change_tx
            && tx.send((state, manual)).is_err()
        {
            tracing::debug!(
                "State change watcher disconnected, could not notify state: {:?}",
                state
            );
        }
    }
}

impl Default for ClientState {
    fn default() -> Self {
        Self::new()
    }
}
