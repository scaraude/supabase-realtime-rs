use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceMeta {
    pub presence_ref: String,
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

pub type PresenceState = HashMap<String, Vec<PresenceMeta>>;

#[derive(Debug, Clone, Deserialize)]
pub struct RawPresenceDiff {
    pub joins: RawPresenceState,
    pub leaves: RawPresenceState,
}

#[derive(Debug, Clone)]
pub struct Presence {
    state: PresenceState,
    pending_diffs: Vec<RawPresenceDiff>,
    join_ref: Option<String>,
}

pub struct PresenceChanges {
    pub joins: PresenceState,
    pub leaves: PresenceState,
}

impl Presence {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            pending_diffs: Vec::new(),
            join_ref: None,
        }
    }

    pub fn state(&self) -> &PresenceState {
        &self.state
    }

    pub fn list(&self) -> Vec<(&String, &Vec<PresenceMeta>)> {
        self.state.iter().collect()
    }

    pub fn in_pending_sync_state(&self, server_join_ref: Option<&str>) -> bool {
        match (&self.join_ref, server_join_ref) {
            (None, _) => true,
            (Some(client_join_ref), Some(server_join_ref)) => server_join_ref != client_join_ref,
            (Some(_), None) => true,
        }
    }

    pub fn add_pending_diff(&mut self, diff: RawPresenceDiff) {
        self.pending_diffs.push(diff);
    }

    pub fn flush_pending_diffs(&mut self) -> Vec<PresenceChanges> {
        let diffs = std::mem::take(&mut self.pending_diffs);
        diffs.into_iter().map(|diff| self.sync_diff(diff)).collect()
    }

    pub fn sync_state(&mut self, new_state: RawPresenceState, join_ref: String) -> PresenceChanges {
        let new_state: PresenceState = Self::transform_state(new_state);
        let new_presence_refs: HashSet<&String> = new_state
            .values()
            .flat_map(|metas| metas.iter().map(|meta| &meta.presence_ref))
            .collect();
        let current_presence_refs: HashSet<&String> = self
            .state
            .values()
            .flat_map(|metas| metas.iter().map(|meta| &meta.presence_ref))
            .collect();

        let joins: PresenceState = new_state
            .iter()
            .filter_map(|(key, metas)| {
                // Filter this user's metas to only new ones
                let new_metas: Vec<PresenceMeta> = metas
                    .iter()
                    .filter(|meta| !current_presence_refs.contains(&meta.presence_ref))
                    .cloned() // Clone to own the data
                    .collect();

                // Only include this user if they have new metas
                if new_metas.is_empty() {
                    None
                } else {
                    Some((key.clone(), new_metas))
                }
            })
            .collect();

        let leaves: PresenceState = self
            .state
            .iter()
            .filter_map(|(key, metas)| {
                // Filter this user's metas to only those that have left
                let left_metas: Vec<PresenceMeta> = metas
                    .iter()
                    .filter(|meta| !new_presence_refs.contains(&meta.presence_ref))
                    .cloned() // Clone to own the data
                    .collect();

                // Only include this user if they have left metas
                if left_metas.is_empty() {
                    None
                } else {
                    Some((key.clone(), left_metas))
                }
            })
            .collect();

        self.state = new_state;
        self.join_ref = Some(join_ref);

        PresenceChanges { joins, leaves }
    }

    pub fn sync_diff(&mut self, diff: RawPresenceDiff) -> PresenceChanges {
        let joins: PresenceState = Self::transform_state(diff.joins.clone());
        let leaves: PresenceState = Self::transform_state(diff.leaves.clone());

        joins.iter().for_each(|(key, metas)| {
            let current_meta = self.state.entry(key.clone()).or_default();

            let new_refs: Vec<&str> = metas
                .iter()
                .map(|meta| meta.presence_ref.as_str())
                .collect();
            current_meta.retain(|meta| !new_refs.contains(&meta.presence_ref.as_str()));
            current_meta.extend_from_slice(metas);
        });

        leaves.iter().for_each(|(key, metas)| {
            if let Some(current_meta) = self.state.get_mut(key) {
                let remove_refs: Vec<&str> = metas
                    .iter()
                    .map(|meta| meta.presence_ref.as_str())
                    .collect();
                current_meta.retain(|meta| !remove_refs.contains(&meta.presence_ref.as_str()));

                if current_meta.is_empty() {
                    self.state.remove(key);
                }
            }
        });

        PresenceChanges { joins, leaves }
    }

    fn transform_state(raw_state: RawPresenceState) -> PresenceState {
        raw_state
            .into_iter()
            .map(|(key, raw_entries)| {
                let entries: Vec<PresenceMeta> = raw_entries
                    .metas
                    .into_iter()
                    .map(|raw_meta| PresenceMeta {
                        presence_ref: raw_meta.phx_ref.unwrap_or_default(),
                        data: raw_meta.data,
                    })
                    .collect();
                (key, entries)
            })
            .collect()
    }
}

impl Default for Presence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_presence_is_empty() {
        let presence = Presence::default();
        assert!(presence.state.is_empty());
        assert!(presence.list().is_empty());
    }

    #[test]
    fn test_sync_state_detects_new_user() {
        let mut presence = Presence::default();

        let new_state: RawPresenceState = HashMap::from([(
            "user-1".to_string(),
            RawPresenceEntries {
                metas: vec![RawPresenceMeta {
                    phx_ref: Some("join-ref".to_string()),
                    phx_ref_prev: None,
                    data: HashMap::new(),
                }],
            },
        )]);

        let changes = presence.sync_state(new_state, "join-ref".to_string());

        assert_eq!(changes.joins.len(), 1);
        assert!(changes.joins.contains_key("user-1"));
        assert_eq!(changes.leaves.len(), 0);

        assert_eq!(presence.state.len(), 1);
        assert!(presence.state.contains_key("user-1"));
    }

    #[test]
    fn test_pending_sync_state() {
        let mut presence = Presence::default();

        assert!(presence.in_pending_sync_state(Some("ref1")));

        let raw_state = HashMap::new();
        presence.sync_state(raw_state, "ref1".to_string());

        assert!(!presence.in_pending_sync_state(Some("ref1")));
        assert!(presence.in_pending_sync_state(Some("ref2")));
    }

    #[test]
    fn test_sync_diff() {
        let mut presence = Presence::default();

        let new_state: RawPresenceState = HashMap::from([(
            "user-1".to_string(),
            RawPresenceEntries {
                metas: vec![RawPresenceMeta {
                    phx_ref: Some("join-ref".to_string()),
                    phx_ref_prev: None,
                    data: HashMap::new(),
                }],
            },
        )]);

        let diff = RawPresenceDiff {
            joins: new_state.clone(),
            leaves: HashMap::new(),
        };

        let changes = presence.sync_diff(diff);

        assert_eq!(changes.joins.len(), 1);
        assert!(changes.joins.contains_key("user-1"));
        assert_eq!(changes.leaves.len(), 0);
    }

    #[test]
    fn test_add_pending_diff() {
        let mut presence = Presence::default();
        let diff = RawPresenceDiff {
            joins: HashMap::new(),
            leaves: HashMap::new(),
        };
        presence.add_pending_diff(diff);
        assert_eq!(presence.pending_diffs.len(), 1);
    }
}
