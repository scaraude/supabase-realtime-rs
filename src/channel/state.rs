use super::push::Push;
use crate::channel::presence::Presence;
use crate::messaging::ChannelEvent;
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

/// Event binding for channel event listeners
#[derive(Debug)]
pub struct EventBinding {
    pub event: ChannelEvent,
    pub filter: Option<HashMap<String, String>>,
    pub sender: mpsc::Sender<serde_json::Value>,
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
