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
                .expect("Failed to serialize event to string")
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
pub struct ColumnInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresChangesPayloadBase {
    pub schema: String,
    pub table: String,
    pub commit_timestamp: String,
    #[serde(default)]
    pub errors: Option<Vec<String>>,
    #[serde(default)]
    pub columns: Vec<ColumnInfo>,
}

// Internal struct for deserializing server format
#[derive(Deserialize, Debug)]
struct PostgreInsertPayloadRaw {
    #[serde(flatten)]
    base: PostgresChangesPayloadBase,
    #[serde(default)]
    record: HashMap<String, Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PostgreInsertPayload {
    #[serde(flatten)]
    pub base: PostgresChangesPayloadBase,
    pub new: HashMap<String, Value>,
}

impl<'de> Deserialize<'de> for PostgreInsertPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = PostgreInsertPayloadRaw::deserialize(deserializer)?;
        Ok(PostgreInsertPayload {
            base: raw.base,
            new: raw.record,
        })
    }
}

// Internal struct for deserializing server format
#[derive(Deserialize, Debug)]
struct PostgresUpdatePayloadRaw {
    #[serde(flatten)]
    base: PostgresChangesPayloadBase,
    #[serde(default)]
    record: HashMap<String, Value>,
    #[serde(default)]
    old_record: HashMap<String, Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PostgresUpdatePayload {
    #[serde(flatten)]
    pub base: PostgresChangesPayloadBase,
    pub new: HashMap<String, Value>,
    pub old: HashMap<String, Value>,
}

impl<'de> Deserialize<'de> for PostgresUpdatePayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = PostgresUpdatePayloadRaw::deserialize(deserializer)?;
        Ok(PostgresUpdatePayload {
            base: raw.base,
            new: raw.record,
            old: raw.old_record,
        })
    }
}

// Internal struct for deserializing server format
#[derive(Deserialize, Debug)]
struct PostgresDeletePayloadRaw {
    #[serde(flatten)]
    base: PostgresChangesPayloadBase,
    #[serde(default)]
    old_record: HashMap<String, Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PostgresDeletePayload {
    #[serde(flatten)]
    pub base: PostgresChangesPayloadBase,
    pub old: HashMap<String, Value>,
}

impl<'de> Deserialize<'de> for PostgresDeletePayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = PostgresDeletePayloadRaw::deserialize(deserializer)?;
        Ok(PostgresDeletePayload {
            base: raw.base,
            old: raw.old_record,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_insert_payload_from_server_format() {
        // This is the actual format sent by Supabase server
        let json = r#"{
            "columns": [
                {"name": "id", "type": "int8"},
                {"name": "created_at", "type": "timestamptz"},
                {"name": "name", "type": "text"}
            ],
            "commit_timestamp": "2025-11-27T16:16:54.545Z",
            "errors": null,
            "record": {
                "created_at": "2025-11-27T16:16:54.541196+00:00",
                "id": 47,
                "name": "jena"
            },
            "schema": "public",
            "table": "users",
            "type": "INSERT"
        }"#;

        let payload: PostgresChangesPayload = serde_json::from_str(json).unwrap();

        match payload {
            PostgresChangesPayload::Insert(insert) => {
                assert_eq!(insert.base.schema, "public");
                assert_eq!(insert.base.table, "users");
                assert_eq!(insert.base.columns.len(), 3);
                assert_eq!(insert.new.get("name").unwrap().as_str().unwrap(), "jena");
                assert_eq!(insert.new.get("id").unwrap().as_i64().unwrap(), 47);
            }
            _ => panic!("Expected Insert variant"),
        }
    }

    #[test]
    fn test_deserialize_update_payload_from_server_format() {
        let json = r#"{
            "columns": [
                {"name": "id", "type": "int8"},
                {"name": "name", "type": "text"}
            ],
            "commit_timestamp": "2025-11-27T16:20:00.000Z",
            "errors": null,
            "record": {
                "id": 47,
                "name": "new_name"
            },
            "old_record": {
                "id": 47,
                "name": "old_name"
            },
            "schema": "public",
            "table": "users",
            "type": "UPDATE"
        }"#;

        let payload: PostgresChangesPayload = serde_json::from_str(json).unwrap();

        match payload {
            PostgresChangesPayload::Update(update) => {
                assert_eq!(update.base.schema, "public");
                assert_eq!(update.base.table, "users");
                assert_eq!(
                    update.new.get("name").unwrap().as_str().unwrap(),
                    "new_name"
                );
                assert_eq!(
                    update.old.get("name").unwrap().as_str().unwrap(),
                    "old_name"
                );
            }
            _ => panic!("Expected Update variant"),
        }
    }

    #[test]
    fn test_deserialize_delete_payload_from_server_format() {
        let json = r#"{
            "columns": [
                {"name": "id", "type": "int8"},
                {"name": "name", "type": "text"}
            ],
            "commit_timestamp": "2025-11-27T16:25:00.000Z",
            "errors": null,
            "old_record": {
                "id": 47,
                "name": "deleted_name"
            },
            "schema": "public",
            "table": "users",
            "type": "DELETE"
        }"#;

        let payload: PostgresChangesPayload = serde_json::from_str(json).unwrap();

        match payload {
            PostgresChangesPayload::Delete(delete) => {
                assert_eq!(delete.base.schema, "public");
                assert_eq!(delete.base.table, "users");
                assert_eq!(
                    delete.old.get("name").unwrap().as_str().unwrap(),
                    "deleted_name"
                );
            }
            _ => panic!("Expected Delete variant"),
        }
    }
}
