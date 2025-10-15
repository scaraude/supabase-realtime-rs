use crate::types::constants::{channel_events, phoenix_events};
use serde::{Deserialize, Serialize};

/// Type-safe channel events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelEvent {
    /// PostgreSQL database changes
    #[serde(rename = "postgres_changes")]
    PostgresChanges,

    /// Custom broadcast event
    Broadcast,

    /// Presence tracking events
    Presence,

    /// System events (phx_*)
    System(SystemEvent),

    /// Custom user-defined event
    Custom(String),
}

impl ChannelEvent {
    /// Parse a string into a ChannelEvent
    pub fn from_str(s: &str) -> Self {
        match s {
            channel_events::POSTGRES_CHANGES => Self::PostgresChanges,
            channel_events::BROADCAST => Self::Broadcast,
            channel_events::PRESENCE => Self::Presence,
            _ if s.starts_with("phx_") || s == phoenix_events::HEARTBEAT => {
                Self::System(SystemEvent::from_str(s))
            }
            _ => Self::Custom(s.to_string()),
        }
    }

    /// Convert event to string representation
    pub fn as_str(&self) -> &str {
        match self {
            Self::PostgresChanges => channel_events::POSTGRES_CHANGES,
            Self::Broadcast => channel_events::BROADCAST,
            Self::Presence => channel_events::PRESENCE,
            Self::System(sys) => sys.as_str(),
            Self::Custom(s) => s,
        }
    }
}

impl From<&str> for ChannelEvent {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<String> for ChannelEvent {
    fn from(s: String) -> Self {
        Self::from_str(&s)
    }
}

impl std::fmt::Display for ChannelEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Phoenix system events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemEvent {
    /// Join channel
    #[serde(rename = "phx_join")]
    Join,

    /// Leave channel
    #[serde(rename = "phx_leave")]
    Leave,

    /// Reply to a message
    #[serde(rename = "phx_reply")]
    Reply,

    /// Close channel
    #[serde(rename = "phx_close")]
    Close,

    /// Error event
    #[serde(rename = "phx_error")]
    Error,

    /// Heartbeat
    #[serde(rename = "heartbeat")]
    Heartbeat,
}

impl SystemEvent {
    pub fn from_str(s: &str) -> Self {
        match s {
            phoenix_events::JOIN => Self::Join,
            phoenix_events::LEAVE => Self::Leave,
            phoenix_events::REPLY => Self::Reply,
            phoenix_events::CLOSE => Self::Close,
            phoenix_events::ERROR => Self::Error,
            phoenix_events::HEARTBEAT => Self::Heartbeat,
            _ => Self::Error, // Default to error for unknown system events
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Join => phoenix_events::JOIN,
            Self::Leave => phoenix_events::LEAVE,
            Self::Reply => phoenix_events::REPLY,
            Self::Close => phoenix_events::CLOSE,
            Self::Error => phoenix_events::ERROR,
            Self::Heartbeat => phoenix_events::HEARTBEAT,
        }
    }
}

/// Postgres change types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PostgresChangeType {
    Insert,
    Update,
    Delete,
}

/// Postgres change event filter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresChangeFilter {
    pub event: PostgresChangeType,
    pub schema: String,
    pub table: String,
    pub filter: Option<String>,
}

impl PostgresChangeFilter {
    pub fn new(
        event: PostgresChangeType,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        Self {
            event,
            schema: schema.into(),
            table: table.into(),
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_event_from_str() {
        assert_eq!(
            ChannelEvent::from_str("postgres_changes"),
            ChannelEvent::PostgresChanges
        );
        assert_eq!(ChannelEvent::from_str("broadcast"), ChannelEvent::Broadcast);
        assert_eq!(ChannelEvent::from_str("presence"), ChannelEvent::Presence);
        assert_eq!(
            ChannelEvent::from_str("phx_join"),
            ChannelEvent::System(SystemEvent::Join)
        );
        assert_eq!(
            ChannelEvent::from_str("my_custom_event"),
            ChannelEvent::Custom("my_custom_event".to_string())
        );
    }

    #[test]
    fn test_system_event_round_trip() {
        let events = vec![
            SystemEvent::Join,
            SystemEvent::Leave,
            SystemEvent::Reply,
            SystemEvent::Close,
            SystemEvent::Error,
            SystemEvent::Heartbeat,
        ];

        for event in events {
            let s = event.as_str();
            assert_eq!(SystemEvent::from_str(s), event);
        }
    }
}
