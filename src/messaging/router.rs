use super::SystemEvent;
use crate::ChannelEvent;
use crate::channel::{RawPresenceDiff, RawPresenceState};
use crate::client::ClientState;
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

        // Handle push replies
        if message.event == ChannelEvent::System(SystemEvent::Reply) {
            if self.handle_push_reply(&message).await {
                return; // Reply was handled, don't route to channels
            }
        }

        if message.event == ChannelEvent::PresenceDiff
            || message.event == ChannelEvent::PresenceState
        {
            self.handle_presence(&message).await;
        }

        // Route to channels
        self.route_to_channels(message).await;
    }

    /// Checks if a message is a heartbeat acknowledgment
    fn is_heartbeat_message(&self, message: &RealtimeMessage) -> bool {
        message.topic == PHOENIX_TOPIC && message.event == ChannelEvent::System(SystemEvent::Reply)
            || message.event == ChannelEvent::System(SystemEvent::Heartbeat)
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

    async fn handle_presence(&self, message: &RealtimeMessage) -> bool {
        let state = self.state.read().await;
        let channels = state
            .channels
            .iter()
            .find(|channel| channel.topic() == message.topic)
            .cloned();
        drop(state);

        let Some(channel) = channels else {
            tracing::warn!(
                "Received presence message for unknown channel topic: {}",
                message.topic
            );
            return false;
        };

        let mut channel_state = channel.state.write().await;

        match message.event {
            ChannelEvent::PresenceState => {
                if let Some(server_presence_state) =
                    serde_json::from_value::<RawPresenceState>(message.payload.clone()).ok()
                {
                    // Get join_ref from channel state (stored during subscribe)
                    let Some(join_ref) = channel_state.join_ref.clone() else {
                        tracing::warn!(
                            "Channel has no join_ref for presence_state on topic: {}",
                            message.topic
                        );
                        return false;
                    };

                    tracing::debug!(
                        "Syncing presence_state for topic {} with join_ref: {}",
                        message.topic,
                        join_ref
                    );

                    channel_state
                        .presence
                        .sync_state(server_presence_state, join_ref);
                    let pending_diffs = channel_state.presence.flush_pending_diffs();
                    if !pending_diffs.is_empty() {
                        tracing::debug!(
                            "Flushed {} pending presence diffs after state sync on topic: {}",
                            pending_diffs.len(),
                            message.topic
                        );
                    }
                    return true;
                }
            }
            ChannelEvent::PresenceDiff => {
                if let Some(server_presence_diff) =
                    serde_json::from_value::<RawPresenceDiff>(message.payload.clone()).ok()
                {
                    let has_pending_sync = channel_state
                        .presence
                        .in_pending_sync_state(channel_state.join_ref.as_deref());
                    if has_pending_sync {
                        channel_state
                            .presence
                            .add_pending_diff(server_presence_diff);
                    } else {
                        channel_state.presence.sync_diff(server_presence_diff);
                    }
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    /// Handles push reply by matching ref to pending push
    /// Returns true if reply was handled
    async fn handle_push_reply(&self, message: &RealtimeMessage) -> bool {
        let Some(ref_id) = &message.r#ref else {
            return false;
        };

        // Find the channel for this topic
        let state = self.state.read().await;
        let channel = state
            .channels
            .iter()
            .find(|ch| ch.topic() == message.topic)
            .cloned();
        drop(state);

        let Some(channel) = channel else {
            return false;
        };

        // Check if this channel has a pending push with this ref
        let mut channel_state = channel.state.write().await;
        let Some(push) = channel_state.pending_pushes.remove(ref_id) else {
            return false;
        };
        drop(channel_state);

        // Extract status from payload
        let status = message
            .payload
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                tracing::debug!("Push reply missing 'status' field, defaulting to 'error'");
                "error"
            });

        let response = message
            .payload
            .get("response")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // Trigger the appropriate callback
        push.trigger(status, response);

        tracing::debug!(
            "Handled push reply for ref {} with status {}",
            ref_id,
            status
        );
        true
    }

    /// Routes message to matching channels
    async fn route_to_channels(&self, message: RealtimeMessage) {
        tracing::debug!(
            "Routing message: topic={}, event={}, payload={}",
            message.topic,
            message.event.as_str(),
            serde_json::to_string(&message.payload).unwrap_or_default()
        );

        let state = self.state.read().await;
        for channel in state.channels.iter() {
            if channel.topic() == message.topic {
                tracing::debug!(
                    "Triggering event {} on channel {}",
                    message.event.as_str(),
                    channel.topic()
                );
                channel
                    ._trigger(message.event.clone(), message.payload.clone())
                    .await;
            }
        }
    }
}
