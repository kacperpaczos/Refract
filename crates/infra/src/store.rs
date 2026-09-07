use anyhow::{Context, Result};
use domain::history::{ChangeKind, HistoryEntry};
use domain::snapshot::Snapshot;
use std::fs;
use std::path::PathBuf;

pub struct Store {
    base_dir: PathBuf,
}

impl Store {
    pub fn new() -> Result<Self> {
        let mut base_dir = dirs::data_local_dir().context("Could not find local data dir")?;
        base_dir.push("desktop-experience-switcher");

        fs::create_dir_all(&base_dir).context("Failed to create base data dir")?;
        fs::create_dir_all(base_dir.join("snapshots")).context("Failed to create snapshots dir")?;

        Ok(Self { base_dir })
    }

    pub fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let path = self
            .base_dir
            .join("snapshots")
            .join(format!("{}.json", snapshot.id));
        let json = serde_json::to_string_pretty(snapshot)?;
        fs::write(&path, json)
            .with_context(|| format!("Failed to write snapshot to {:?}", path))?;
        Ok(())
    }

    pub fn read_snapshot(&self, id: &uuid::Uuid) -> Result<Snapshot> {
        let path = self.base_dir.join("snapshots").join(format!("{}.json", id));
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read snapshot from {:?}", path))?;
        let snapshot: Snapshot = serde_json::from_str(&content)?;
        Ok(snapshot)
    }

    pub fn delete_snapshot(&self, id: &uuid::Uuid) -> Result<()> {
        let path = self.base_dir.join("snapshots").join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete snapshot from {:?}", path))?;
        }
        Ok(())
    }

    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        let mut snapshots = Vec::new();
        let snapshots_dir = self.base_dir.join("snapshots");
        
        if !snapshots_dir.exists() {
            return Ok(snapshots);
        }

        for entry in fs::read_dir(snapshots_dir)? {
            let entry = entry?;
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let content = fs::read_to_string(entry.path())?;
                if let Ok(snapshot) = serde_json::from_str::<Snapshot>(&content) {
                    snapshots.push(snapshot);
                }
            }
        }
        
        snapshots.sort_by_key(|s| std::cmp::Reverse(s.timestamp));
        Ok(snapshots)
    }

    pub fn append_history(&self, entry: &HistoryEntry) -> Result<()> {
        let path = self.base_dir.join("history.json");
        let mut history = self.read_history().unwrap_or_default();
        history.push(entry.clone());
        let json = serde_json::to_string_pretty(&history)?;
        fs::write(&path, json).with_context(|| format!("Failed to write history to {:?}", path))?;
        Ok(())
    }

    pub fn read_history(&self) -> Result<Vec<HistoryEntry>> {
        let path = self.base_dir.join("history.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read history from {:?}", path))?;
        let history: Vec<HistoryEntry> = serde_json::from_str(&content)?;
        Ok(history)
    }

    pub fn remove_history_entries_for_snapshot(&self, id: &uuid::Uuid) -> Result<()> {
        let path = self.base_dir.join("history.json");
        let mut history = self.read_history().unwrap_or_default();
        history.retain(|entry| !history_entry_references_snapshot(entry, id));
        let json = serde_json::to_string_pretty(&history)?;
        fs::write(&path, json).with_context(|| format!("Failed to write history to {:?}", path))?;
        Ok(())
    }
}

fn history_entry_references_snapshot(entry: &HistoryEntry, id: &uuid::Uuid) -> bool {
    if entry.snapshot_id_before == Some(*id) || entry.snapshot_id_after == Some(*id) {
        return true;
    }

    matches!(
        entry.kind,
        ChangeKind::SnapshotRestored(snapshot_id) | ChangeKind::ManualSnapshot(snapshot_id) if snapshot_id == *id
    )
}
