use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMessage {
    pub topic: String,
    pub event: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_ref: Option<String>,
}

impl RealtimeMessage {
    pub fn new(topic: String, event: String, payload: serde_json::Value) -> Self {
        Self {
            topic,
            event,
            payload,
            r#ref: None,
            join_ref: None,
        }
    }

    pub fn with_ref(mut self, r#ref: String) -> Self {
        self.r#ref = Some(r#ref);
        self
    }

    pub fn with_join_ref(mut self, join_ref: String) -> Self {
        self.join_ref = Some(join_ref);
        self
    }
}
