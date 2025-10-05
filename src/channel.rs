use crate::client::RealtimeClient;
use crate::types::{ChannelState, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct RealtimeChannelOptions {
    pub broadcast_self: bool,
    pub broadcast_ack: bool,
    pub presence_key: Option<String>,
    pub is_private: bool,
}

pub struct RealtimeChannel {
    topic: String,
    client: Arc<RealtimeClient>,
    state: Arc<RwLock<ChannelState>>,
    options: RealtimeChannelOptions,
}

impl RealtimeChannel {
    pub fn new(
        topic: String,
        client: Arc<RealtimeClient>,
        options: RealtimeChannelOptions,
    ) -> Self {
        Self {
            topic,
            client,
            state: Arc::new(RwLock::new(ChannelState::Closed)),
            options,
        }
    }

    /// Subscribe to the channel
    pub async fn subscribe(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if *state == ChannelState::Joined || *state == ChannelState::Joining {
            return Ok(());
        }

        *state = ChannelState::Joining;

        // TODO: Implement channel subscription
        tracing::info!("Subscribing to channel: {}", self.topic);

        *state = ChannelState::Joined;

        Ok(())
    }

    /// Unsubscribe from the channel
    pub async fn unsubscribe(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if *state == ChannelState::Closed {
            return Ok(());
        }

        *state = ChannelState::Leaving;

        // TODO: Implement channel unsubscription

        *state = ChannelState::Closed;

        Ok(())
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}
