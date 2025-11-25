use crate::ChannelEvent;
use crate::types::{RealtimeError, error::Result};
use serde_json::Value;

/// Handles HTTP broadcast fallback when WebSocket is unavailable
pub struct HttpBroadcaster {
    base_endpoint: String,
    api_key: String,
    access_token: Option<String>,
}

impl HttpBroadcaster {
    pub fn new(base_endpoint: String, api_key: String, access_token: Option<String>) -> Self {
        Self {
            base_endpoint,
            api_key,
            access_token,
        }
    }

    /// Sends a broadcast message via HTTP POST
    pub async fn broadcast(
        &self,
        topic: &str,
        event: ChannelEvent,
        payload: Value,
        is_private: bool,
    ) -> Result<()> {
        let url = format!("{}/api/broadcast", self.base_endpoint);

        let body = serde_json::json!({
            "messages": [{
                "topic": topic,
                "event": event,
                "payload": payload,
                "private": is_private,
            }]
        });

        let http_client = reqwest::Client::new();
        let mut request = http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("apiKey", &self.api_key)
            .json(&body);

        if let Some(token) = &self.access_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RealtimeError::Connection(format!("HTTP broadcast failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RealtimeError::Connection(format!(
                "HTTP broadcast failed for topic '{}' event '{}' with status: {}",
                topic,
                event.as_str(),
                response.status()
            )));
        }

        tracing::debug!("Sent broadcast via HTTP: {}", event.as_str());
        Ok(())
    }
}

/// Converts WebSocket endpoint to HTTP endpoint
pub fn ws_to_http_endpoint(ws_endpoint: &str) -> String {
    ws_endpoint
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .split('?')
        .next()
        .unwrap_or(ws_endpoint)
        .to_string()
}
