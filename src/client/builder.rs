use super::{ClientState, ConnectionManager, ConnectionState, RealtimeClient};
use crate::types::{RealtimeError, Result};
use std::sync::Arc;
use tokio::sync::{RwLock, watch};

#[derive(Debug, Clone)]
pub struct RealtimeClientOptions {
    pub api_key: String,
    pub timeout: Option<u64>,
    pub heartbeat_interval: Option<u64>,
    pub access_token: Option<String>,
}

impl Default for RealtimeClientOptions {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            timeout: None,
            heartbeat_interval: None,
            access_token: None,
        }
    }
}

/// Builder for RealtimeClient that handles initialization
pub struct RealtimeClientBuilder {
    endpoint: String,
    options: RealtimeClientOptions,
}

impl RealtimeClientBuilder {
    /// Create a new builder
    pub fn new(endpoint: impl Into<String>, options: RealtimeClientOptions) -> Result<Self> {
        let endpoint = endpoint.into();

        // Validate API key is provided
        if options.api_key.is_empty() {
            return Err(RealtimeError::Auth("API key is required".to_string()));
        }

        Ok(Self { endpoint, options })
    }

    /// Build the client and spawn background tasks
    pub fn build(self) -> RealtimeClient {
        let mut client_state = ClientState::new();

        // Initialize state watcher channel
        let (state_tx, state_rx) = watch::channel((ConnectionState::Closed, false));
        client_state.state_change_tx = Some(state_tx);

        let client = RealtimeClient {
            endpoint: self.endpoint,
            options: self.options,
            connection: Arc::new(ConnectionManager::new()),
            state: Arc::new(RwLock::new(client_state)),
        };

        // Spawn reconnection watcher task
        let client_for_watcher = client.clone();
        tokio::spawn(async move {
            let mut rx = state_rx;

            while rx.changed().await.is_ok() {
                let (state, was_manual) = *rx.borrow_and_update();

                // Reconnect if closed/disconnected AND not manual
                if matches!(state, ConnectionState::Closed) && !was_manual {
                    tracing::info!("State watcher detected disconnect, attempting reconnection...");

                    if let Err(e) = client_for_watcher.try_reconnect().await {
                        tracing::error!("Reconnection watcher failed: {}", e);
                    }
                }
            }
            tracing::info!("Reconnection watcher task finished");
        });

        client
    }
}
