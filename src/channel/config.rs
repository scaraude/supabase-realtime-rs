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
