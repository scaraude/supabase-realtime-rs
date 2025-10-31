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

pub struct PresenceChanges {
    pub joins: PresenceState,
    pub leaves: PresenceState,
}

impl Presence {
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
