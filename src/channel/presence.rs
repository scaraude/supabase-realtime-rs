use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceMeta {
    pub presence_ref: String,
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPresenceMeta {
    pub phx_ref: Option<String>,
    pub phx_ref_prev: Option<String>,
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPresenceEntries {
    pub metas: Vec<RawPresenceMeta>,
}

pub type RawPresenceState = HashMap<String, RawPresenceEntries>;

pub type PresenceState = HashMap<String, Vec<PresenceMeta>>;

#[derive(Debug, Clone, Deserialize)]
pub struct PresenceDiff {
    pub joins: RawPresenceState,
    pub leaves: RawPresenceState,
}

#[derive(Debug, Clone)]
pub struct Presence {
    state: PresenceState,
    pending_diffs: Vec<PresenceDiff>,
    join_ref: Option<String>,
}
