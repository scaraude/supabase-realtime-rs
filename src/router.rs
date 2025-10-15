use crate::SystemEvent;
use crate::client_state::ClientState;
use crate::types::constants::PHOENIX_TOPIC;
use crate::types::message::RealtimeMessage;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Routes incoming messages to appropriate handlers
pub struct MessageRouter {
    state: Arc<RwLock<ClientState>>,
}

impl MessageRouter {
    pub fn new_with_state(state: Arc<RwLock<ClientState>>) -> Self {
        Self { state }
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
        message.topic == PHOENIX_TOPIC
            && (message.event == SystemEvent::Reply || message.event == SystemEvent::Heartbeat)
    }

    /// Handles heartbeat acknowledgment by clearing pending ref
    async fn handle_heartbeat_ack(&self, message: &RealtimeMessage) {
        if let Some(ref msg_ref) = message.r#ref {
            let state = self.state.read().await;
            if state.pending_heartbeat_ref.as_ref() == Some(msg_ref) {
                drop(state);
                self.state.write().await.pending_heartbeat_ref = None;
                tracing::debug!("Received heartbeat ack for ref {}", msg_ref);
            }
        }
    }

    /// Routes message to matching channels
    async fn route_to_channels(&self, message: RealtimeMessage) {
        let state = self.state.read().await;
        for channel in state.channels.iter() {
            if channel.topic() == message.topic {
                channel
                    ._trigger(message.event.clone(), message.payload.clone())
                    .await;
            }
        }
    }
}
