use super::push::Push;
use crate::channel::PostgresChangesPayload;
use crate::channel::presence::Presence;
use crate::messaging::ChannelEvent;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Channel status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelStatus {
    Closed,
    Errored,
    Joined,
    Joining,
    Leaving,
}

/// Typed event payloads for different channel events
#[derive(Debug, Clone, Serialize)]
pub enum EventPayload {
    /// Postgres database change events (INSERT, UPDATE, DELETE)
    PostgresChanges(PostgresChangesPayload),
    /// Broadcast messages (user-defined pub/sub)
    Broadcast(serde_json::Value),
    /// Presence state (full list of present users)
    PresenceState(serde_json::Value),
    /// Presence diff (joins/leaves)
    PresenceDiff(serde_json::Value),
    /// System events (replies, errors, etc.)
    System(serde_json::Value),
    /// Custom user-defined events
    Custom(serde_json::Value),
}

/// Event binding for channel event listeners
#[derive(Debug)]
pub struct EventBinding {
    pub event: ChannelEvent,
    pub filter: Option<HashMap<String, String>>,
    pub sender: mpsc::Sender<EventPayload>,
}

/// Mutable state for a RealtimeChannel
pub struct ChannelState {
    pub status: ChannelStatus,
    pub bindings: Vec<EventBinding>,
    pub pending_pushes: HashMap<String, Arc<Push>>,
    pub presence: Presence,
    pub join_ref: Option<String>,
}

impl ChannelState {
    pub fn new() -> Self {
        Self {
            status: ChannelStatus::Closed,
            bindings: Vec::new(),
            pending_pushes: HashMap::new(),
            presence: Presence::default(),
            join_ref: None,
        }
    }
}

impl Default for ChannelState {
    fn default() -> Self {
        Self::new()
    }
}
