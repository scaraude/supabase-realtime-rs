use crate::channel::RealtimeChannel;
use crate::types::message::RealtimeMessage;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Routes incoming messages to appropriate handlers
pub struct MessageRouter {
    channels: Arc<RwLock<Vec<Arc<RealtimeChannel>>>>,
    pending_heartbeat_ref: Arc<RwLock<Option<String>>>,
}

impl MessageRouter {
    pub fn new(
        channels: Arc<RwLock<Vec<Arc<RealtimeChannel>>>>,
        pending_heartbeat_ref: Arc<RwLock<Option<String>>>,
    ) -> Self {
        Self {
            channels,
            pending_heartbeat_ref,
        }
    }

    /// Routes a message to the appropriate handler(s)
    pub async fn route(&self, message: RealtimeMessage) {
        // Handle heartbeat acknowledgment
        if self.is_heartbeat_message(&message) {
            self.handle_heartbeat_ack(&message).await;
        }

        // Route to channels
        self.route_to_channels(message).await;
    }

    /// Checks if a message is a heartbeat acknowledgment
    fn is_heartbeat_message(&self, message: &RealtimeMessage) -> bool {
        message.topic == "phoenix"
            && (message.event == "phx_reply" || message.event == "heartbeat")
    }

    /// Handles heartbeat acknowledgment by clearing pending ref
    async fn handle_heartbeat_ack(&self, message: &RealtimeMessage) {
        if let Some(ref msg_ref) = message.r#ref {
            let pending = self.pending_heartbeat_ref.read().await;
            if pending.as_ref() == Some(msg_ref) {
                drop(pending);
                *self.pending_heartbeat_ref.write().await = None;
                tracing::debug!("Received heartbeat ack for ref {}", msg_ref);
            }
        }
    }

    /// Routes message to matching channels
    async fn route_to_channels(&self, message: RealtimeMessage) {
        let channels = self.channels.read().await;
        for channel in channels.iter() {
            if channel.topic() == message.topic {
                channel
                    ._trigger(&message.event, message.payload.clone())
                    .await;
            }
        }
    }
}
