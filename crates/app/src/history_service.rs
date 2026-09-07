use anyhow::Result;
use domain::history::HistoryEntry;
use infra::logging::{log_enter, log_ok, preview_debug};
use infra::store::Store;

pub struct HistoryService {
    store: Store,
}

impl HistoryService {
    pub fn new() -> Result<Self> {
        let started_at = log_enter("HistoryService::new", "");
        let service = Self {
            store: Store::new()?,
        };
        log_ok("HistoryService::new", started_at, "status=success");
        Ok(service)
    }

    pub fn log_change(&self, entry: HistoryEntry) -> Result<()> {
        let started_at = log_enter("HistoryService::log_change", &format!("entry={}", preview_debug(&entry)));
        self.store.append_history(&entry)?;
        log_ok("HistoryService::log_change", started_at, "status=success");
        Ok(())
    }

    pub fn get_history(&self) -> Result<Vec<HistoryEntry>> {
        let started_at = log_enter("HistoryService::get_history", "");
        let mut history = self.store.read_history()?;
        history.sort_by_key(|h| std::cmp::Reverse(h.timestamp));
        log_ok("HistoryService::get_history", started_at, &format!("count={}", history.len()));
        Ok(history)
    }

    pub fn delete_entries_for_snapshot(&self, id: &uuid::Uuid) -> Result<()> {
        let started_at = log_enter("HistoryService::delete_entries_for_snapshot", &format!("id={id}"));
        self.store.remove_history_entries_for_snapshot(id)?;
        log_ok("HistoryService::delete_entries_for_snapshot", started_at, "status=success");
        Ok(())
    }
}
