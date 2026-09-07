use domain::layout::{ApplyLayoutReport, PreflightReport};
use anyhow::Result;
use infra::logging::{log_enter, log_ok, preview_debug};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
struct TrackedState {
    pub layout_id: Option<String>,
    pub desktop_key: Option<String>,
    pub provider: Option<String>,
    pub variant: Option<String>,
    pub installed_extensions: std::collections::HashSet<String>,
    pub disabled_extensions: std::collections::HashSet<String>,
    pub snapshot_before: Option<String>,
    pub snapshot_after: Option<String>,
    pub last_preflight_report: Option<PreflightReport>,
    pub last_apply_report: Option<ApplyLayoutReport>,
    pub last_execution_plan_summary: Option<String>,
}

pub struct LayoutTracker {
    tracker_file: PathBuf,
}

impl LayoutTracker {
    pub fn new() -> Result<Self> {
        let started_at = log_enter("LayoutTracker::new", "");
        let mut base_dir = dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("Brak dostępu do katalogu ~/.local/share"))?;
        
        base_dir.push("desktop-experience-switcher");
        fs::create_dir_all(&base_dir)?;
        
        let tracker_file = base_dir.join("current_layout.json");

        let tracker = Self {
            tracker_file,
        };
        log_ok("LayoutTracker::new", started_at, &format!("tracker_file={:?}", tracker.tracker_file));
        Ok(tracker)
    }

    /// Pobiera identyfikatory rozszerzeń dodanych przez populację poprzedniego układu
    pub fn get_installed_extensions(&self) -> std::collections::HashSet<String> {
        let started_at = log_enter(
            "LayoutTracker::get_installed_extensions",
            &format!("tracker_file={:?}", self.tracker_file),
        );
        if self.tracker_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.tracker_file) {
                if let Ok(state) = serde_json::from_str::<TrackedState>(&content) {
                    log_ok(
                        "LayoutTracker::get_installed_extensions",
                        started_at,
                        &format!("extensions={}", preview_debug(&state.installed_extensions)),
                    );
                    return state.installed_extensions;
                }
            }
        }
        log_ok("LayoutTracker::get_installed_extensions", started_at, "extensions=[]");
        std::collections::HashSet::new()
    }

    /// Zapisuje bieżący stan narzuconych na środowisko rozszerzeń (po to by je wyłączyć/usunąć dla innego w przyszłości)
    pub fn save_active_layout(&self, layout_id: &str, extensions: std::collections::HashSet<String>) -> Result<()> {
        self.save_state(TrackedState {
            layout_id: Some(layout_id.to_string()),
            desktop_key: None,
            provider: None,
            variant: None,
            installed_extensions: extensions,
            disabled_extensions: std::collections::HashSet::new(),
            snapshot_before: None,
            snapshot_after: None,
            last_preflight_report: None,
            last_apply_report: None,
            last_execution_plan_summary: None,
        })
    }

    pub fn save_deployment_state(
        &self,
        layout_id: &str,
        desktop_key: &str,
        provider: Option<String>,
        variant: Option<String>,
        installed_extensions: std::collections::HashSet<String>,
        disabled_extensions: std::collections::HashSet<String>,
        snapshot_before: Option<String>,
        snapshot_after: Option<String>,
        last_preflight_report: Option<PreflightReport>,
        last_apply_report: Option<ApplyLayoutReport>,
        last_execution_plan_summary: Option<String>,
    ) -> Result<()> {
        self.save_state(TrackedState {
            layout_id: Some(layout_id.to_string()),
            desktop_key: Some(desktop_key.to_string()),
            provider,
            variant,
            installed_extensions,
            disabled_extensions,
            snapshot_before,
            snapshot_after,
            last_preflight_report,
            last_apply_report,
            last_execution_plan_summary,
        })
    }

    pub fn read_state_json(&self) -> Result<Option<serde_json::Value>> {
        if !self.tracker_file.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.tracker_file)?;
        let value = serde_json::from_str::<serde_json::Value>(&content)?;
        Ok(Some(value))
    }

    fn save_state(&self, state: TrackedState) -> Result<()> {
        let started_at = log_enter(
            "LayoutTracker::save_state",
            &format!(
                "layout_id={:?} installed_extensions={}",
                state.layout_id,
                preview_debug(&state.installed_extensions)
            ),
        );
        fs::write(&self.tracker_file, serde_json::to_string_pretty(&state)?)?;
        log_ok(
            "LayoutTracker::save_state",
            started_at,
            &format!("tracker_file={:?}", self.tracker_file),
        );
        Ok(())
    }
}
