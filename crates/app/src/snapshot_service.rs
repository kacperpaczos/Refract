use anyhow::Result;
use domain::snapshot::{Snapshot, SnapshotKind};
use infra::logging::{log_enter, log_ok};
use infra::store::Store;
use infra::{dconf, extensions};

pub struct SnapshotService {
    store: Store,
}

impl SnapshotService {
    pub fn new() -> Result<Self> {
        let started_at = log_enter("SnapshotService::new", "");
        let service = Self {
            store: Store::new()?,
        };
        log_ok("SnapshotService::new", started_at, "status=success");
        Ok(service)
    }

    pub fn create_snapshot(&self, kind: SnapshotKind) -> Result<Snapshot> {
        let started_at = log_enter("SnapshotService::create_snapshot", &format!("kind={kind:?}"));
        let dump = dconf::dump("/org/gnome/")?;
        let enabled_exts = extensions::list_enabled();
        
        let snapshot = Snapshot::new(kind, dump, enabled_exts);
        self.store.save_snapshot(&snapshot)?;
        log_ok(
            "SnapshotService::create_snapshot",
            started_at,
            &format!("snapshot_id={}", snapshot.id),
        );
        Ok(snapshot)
    }

    pub fn restore_snapshot(&self, id: &uuid::Uuid) -> Result<()> {
        let started_at = log_enter("SnapshotService::restore_snapshot", &format!("id={}", id));
        let snapshot = self.store.read_snapshot(id)?;
        
        // 1. Restore dconf
        dconf::load("/org/gnome/", &snapshot.dconf_dump)?;
        
        // 2. Restore extensions
        let current_exts = extensions::list_enabled();
        
        // Disable what's currently enabled but wasn't in snapshot
        for ext in &current_exts {
            if !snapshot.enabled_extensions.contains(ext) {
                let _ = extensions::disable_extension(ext);
            }
        }
        
        // Enable what was in snapshot but is currently disabled
        for ext in &snapshot.enabled_extensions {
            if !current_exts.contains(ext) {
                let _ = extensions::enable_extension(ext);
            }
        }
        
        log_ok("SnapshotService::restore_snapshot", started_at, "status=success");
        Ok(())
    }

    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        let started_at = log_enter("SnapshotService::list_snapshots", "");
        let snapshots = self.store.list_snapshots()?;
        log_ok("SnapshotService::list_snapshots", started_at, &format!("count={}", snapshots.len()));
        Ok(snapshots)
    }

    pub fn delete_snapshot(&self, id: &uuid::Uuid) -> Result<()> {
        let started_at = log_enter("SnapshotService::delete_snapshot", &format!("id={}", id));
        self.store.delete_snapshot(id)?;
        log_ok("SnapshotService::delete_snapshot", started_at, "status=success");
        Ok(())
    }
}
