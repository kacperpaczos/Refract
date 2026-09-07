use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapshotKind {
    OnLaunch,
    BeforeChange,
    AfterChange,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub kind: SnapshotKind,
    pub dconf_dump: String,
    pub enabled_extensions: Vec<String>,
}

impl Snapshot {
    pub fn new(kind: SnapshotKind, dconf_dump: String, enabled_extensions: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            kind,
            dconf_dump,
            enabled_extensions,
        }
    }
}
