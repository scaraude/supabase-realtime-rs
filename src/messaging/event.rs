use crate::types::constants::{channel_events, phoenix_events};
use serde::{Deserialize, Serialize};

/// Type-safe channel events
///
/// Serializes as plain strings: "broadcast", "phx_reply", "custom_event", etc.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChannelEvent {
    /// PostgreSQL database changes
    PostgresChanges,

    /// Broadcast event with optional inner event name
    /// - `Broadcast(None)` matches all broadcast events
    /// - `Broadcast(Some("message"))` matches only broadcasts with event="message"
    Broadcast(Option<String>),

    /// Presence tracking events
    PresenceState,
    PresenceDiff,

    /// System events (phx_*)
    System(SystemEvent),

    /// Custom user-defined event
    Custom(String),
}

// Custom serialization to serialize as string
impl Serialize for ChannelEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

// Custom deserialization to parse from string
impl<'de> Deserialize<'de> for ChannelEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::parse(&s))
    }
}

impl ChannelEvent {
    /// Parse a string into a ChannelEvent
    pub fn parse(s: &str) -> Self {
        match s {
            channel_events::POSTGRES_CHANGES => Self::PostgresChanges,
            channel_events::BROADCAST => Self::Broadcast(None),
            channel_events::PRESENCE_STATE => Self::PresenceState,
            channel_events::PRESENCE_DIFF => Self::PresenceDiff,
            _ if s.starts_with("phx_") || s == phoenix_events::HEARTBEAT => {
                Self::System(SystemEvent::parse(s))
            }
            _ => Self::Custom(s.to_string()),
        }
    }

    /// Convert event to string representation
    pub fn as_str(&self) -> &str {
        match self {
            Self::PostgresChanges => channel_events::POSTGRES_CHANGES,
            Self::Broadcast(_) => channel_events::BROADCAST,
            Self::PresenceState => channel_events::PRESENCE_STATE,
            Self::PresenceDiff => channel_events::PRESENCE_DIFF,
            Self::System(sys) => sys.as_str(),
            Self::Custom(s) => s,
        }
    }

    /// Create a broadcast event with a specific event name
    ///
    /// # Example
    /// ```
    /// use supabase_realtime_rs::ChannelEvent;
    ///
    /// let event = ChannelEvent::broadcast("message");
    /// ```
    pub fn broadcast(event: impl Into<String>) -> Self {
        Self::Broadcast(Some(event.into()))
    }
}

impl From<&str> for ChannelEvent {
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

impl From<String> for ChannelEvent {
    fn from(s: String) -> Self {
        Self::parse(&s)
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
    pub fn parse(s: &str) -> Self {
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
            ChannelEvent::parse("postgres_changes"),
            ChannelEvent::PostgresChanges
        );
        assert_eq!(
            ChannelEvent::parse("broadcast"),
            ChannelEvent::Broadcast(None)
        );
        assert_eq!(
            ChannelEvent::parse("presence_state"),
            ChannelEvent::PresenceState
        );
        assert_eq!(
            ChannelEvent::parse("presence_diff"),
            ChannelEvent::PresenceDiff
        );
        assert_eq!(
            ChannelEvent::parse("phx_join"),
            ChannelEvent::System(SystemEvent::Join)
        );
        assert_eq!(
            ChannelEvent::parse("my_custom_event"),
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
            assert_eq!(SystemEvent::parse(s), event);
        }
    }
}
