use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeKind {
    AppLaunch,
    LayoutApplied(String),
    LayoutApplyBlocked(String),
    LayoutApplyFailed(String),
    LayoutRolledBack(String),
    SnapshotRestored(Uuid),
    ManualSnapshot(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub kind: ChangeKind,
    pub snapshot_id_before: Option<Uuid>,
    pub snapshot_id_after: Option<Uuid>,
}

impl HistoryEntry {
    pub fn new(
        kind: ChangeKind,
        snapshot_id_before: Option<Uuid>,
        snapshot_id_after: Option<Uuid>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            kind,
            snapshot_id_before,
            snapshot_id_after,
        }
    }
}
