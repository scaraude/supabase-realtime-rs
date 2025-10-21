use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PostgresChangeEvent {
    #[serde(rename = "*")]
    All,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresChangesFilter {
    pub event: PostgresChangeEvent,
    pub schema: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

impl PostgresChangesFilter {
    pub fn new(event: PostgresChangeEvent, schema: impl Into<String>) -> Self {
        Self {
            event,
            schema: schema.into(),
            table: None,
            filter: None,
        }
    }

    pub fn table(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }
    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    pub fn to_hash_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(
            "event".to_string(),
            serde_json::to_string(&self.event)
                .unwrap()
                .replace("\"", ""),
        );
        map.insert("schema".to_string(), self.schema.clone());
        if let Some(table) = &self.table {
            map.insert("table".to_string(), table.clone());
        }
        if let Some(filter) = &self.filter {
            map.insert("filter".to_string(), filter.clone());
        }
        map
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresChangesPayloadBase {
    pub schema: String,
    pub table: String,
    pub commit_timestamp: String,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgreInsertPayload {
    #[serde(flatten)]
    pub base: PostgresChangesPayloadBase,
    #[serde(default)]
    pub new: HashMap<String, Value>,
    #[serde(default)]
    pub old: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresUpdatePayload {
    #[serde(flatten)]
    pub base: PostgresChangesPayloadBase,
    #[serde(default)]
    pub new: HashMap<String, Value>,
    #[serde(default)]
    pub old: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresDeletePayload {
    #[serde(flatten)]
    pub base: PostgresChangesPayloadBase,
    #[serde(default)]
    pub new: HashMap<String, Value>,
    #[serde(default)]
    pub old: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "eventType", rename_all = "UPPERCASE")]
pub enum PostgresChangesPayload {
    Insert(PostgreInsertPayload),
    Update(PostgresUpdatePayload),
    Delete(PostgresDeletePayload),
}

impl PostgresChangesPayload {
    pub fn schema(&self) -> &str {
        match self {
            Self::Insert(payload) => &payload.base.schema,
            Self::Update(payload) => &payload.base.schema,
            Self::Delete(payload) => &payload.base.schema,
        }
    }

    pub fn table(&self) -> &str {
        match self {
            Self::Insert(payload) => &payload.base.table,
            Self::Update(payload) => &payload.base.table,
            Self::Delete(payload) => &payload.base.table,
        }
    }

    pub fn commit_timestamp(&self) -> &str {
        match self {
            Self::Insert(payload) => &payload.base.commit_timestamp,
            Self::Update(payload) => &payload.base.commit_timestamp,
            Self::Delete(payload) => &payload.base.commit_timestamp,
        }
    }
}
