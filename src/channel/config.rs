use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BroadcastConfig {
    /// Enable client to receive messages it broadcast
    #[serde(rename = "self")]
    pub self_: bool,
    /// Instruct server to acknowledge broadcast receipt
    pub ack: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PresenceConfig {
    /// Track presence payload across clients
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Enable presence tracking
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostgresChangesConfig {
    pub event: String, // "*" | "INSERT" | "UPDATE" | "DELETE"
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// Channel join payload configuration
/// Reference: https://supabase.com/docs/guides/realtime/protocol
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelJoinConfig {
    pub broadcast: BroadcastConfig,
    pub presence: PresenceConfig,
    #[serde(rename = "private")]
    pub is_private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres_changes: Option<Vec<PostgresChangesConfig>>,
}

/// Full join payload sent to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinPayload {
    pub config: ChannelJoinConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

/// Postgres changes event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PostgresEventType {
    Insert,
    Update,
    Delete,
}

/// Postgres changes data (inside message payload)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresChangesData {
    #[serde(rename = "type")]
    pub event_type: PostgresEventType,
    pub schema: String,
    pub table: String,
    pub commit_timestamp: String,
    #[serde(default)]
    pub errors: Option<Vec<String>>,
    /// New record data (for INSERT/UPDATE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new: Option<serde_json::Value>,
    /// Old record data (for UPDATE/DELETE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old: Option<serde_json::Value>,
}

/// Full postgres changes message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresChangesPayload {
    pub ids: Vec<i32>,
    pub data: PostgresChangesData,
}
