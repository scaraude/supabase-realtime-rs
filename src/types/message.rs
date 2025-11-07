use serde::{Deserialize, Serialize};

use crate::ChannelEvent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeMessage {
    pub topic: String,
    pub event: ChannelEvent,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_ref: Option<String>,
}

impl RealtimeMessage {
    pub fn new(topic: String, event: ChannelEvent, payload: serde_json::Value) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realtime_message() {
        let message = RealtimeMessage::new(
            "test".to_string(),
            ChannelEvent::Custom("message".to_string()),
            serde_json::Value::Null,
        );
        assert_eq!(message.topic, "test");
        assert_eq!(message.event, ChannelEvent::Custom("message".to_string()));
        assert_eq!(message.payload, serde_json::Value::Null);
        assert_eq!(message.r#ref, None);
        assert_eq!(message.join_ref, None);
    }

    #[test]
    fn test_realtime_message_round_trip() {
        let message = RealtimeMessage::new(
            "test".to_string(),
            ChannelEvent::Custom("message".to_string()),
            serde_json::Value::Null,
        )
        .with_ref("1".to_string())
        .with_join_ref("321".to_string());

        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: RealtimeMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_realtime_message_serialization_without_ref_and_join_ref() {
        let message = RealtimeMessage::new(
            "test".to_string(),
            ChannelEvent::Custom("message".to_string()),
            serde_json::Value::Null,
        );

        let json = serde_json::to_string(&message).unwrap();
        assert!(!json.contains(r#""ref":"#));
        assert!(!json.contains(r#""join_ref":"#));
    }

    #[test]
    fn test_realtime_message_serialization_with_ref_and_join_ref() {
        let message = RealtimeMessage::new(
            "test".to_string(),
            ChannelEvent::Custom("message".to_string()),
            serde_json::Value::Null,
        )
        .with_ref("123".to_string())
        .with_join_ref("321".to_string());

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains(r#""ref":"123""#));
        assert!(json.contains(r#""join_ref":"321""#));
    }
}
