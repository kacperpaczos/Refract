use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyProgress {
    pub layout_id: String,
    pub total_steps: usize,
    pub current_step: usize,
    pub phase: String,
    #[serde(default)]
    pub stage_label: String,
    #[serde(default)]
    pub current_target: Option<String>,
    pub detail: String,
    #[serde(default)]
    pub updated_at_unix_ms: u64,
    pub finished: bool,
    pub success: bool,
}

#[derive(Clone)]
pub struct ProgressReporter {
    path: Option<PathBuf>,
    layout_id: String,
    total_steps: usize,
}

impl ProgressReporter {
    pub fn from_env(layout_id: &str, total_steps: usize) -> Self {
        let path = std::env::var("DESKTOP_EXPERIENCE_PROGRESS_FILE")
            .ok()
            .map(PathBuf::from);
        Self {
            path,
            layout_id: layout_id.to_string(),
            total_steps,
        }
    }

    pub fn update(&self, current_step: usize, phase: &str, detail: &str) {
        self.update_stage(current_step, phase, detail, None, detail);
    }

    pub fn update_stage(
        &self,
        current_step: usize,
        phase: &str,
        stage_label: &str,
        current_target: Option<&str>,
        detail: &str,
    ) {
        let _ = self.write(ApplyProgress {
            layout_id: self.layout_id.clone(),
            total_steps: self.total_steps,
            current_step,
            phase: phase.to_string(),
            stage_label: stage_label.to_string(),
            current_target: current_target.map(|target| target.to_string()),
            detail: detail.to_string(),
            updated_at_unix_ms: current_timestamp_ms(),
            finished: false,
            success: false,
        });
    }

    pub fn finish(&self, success: bool, detail: &str) {
        let _ = self.write(ApplyProgress {
            layout_id: self.layout_id.clone(),
            total_steps: self.total_steps,
            current_step: self.total_steps,
            phase: if success { "done" } else { "error" }.to_string(),
            stage_label: if success { "Layout applied" } else { "Layout apply failed" }.to_string(),
            current_target: None,
            detail: detail.to_string(),
            updated_at_unix_ms: current_timestamp_ms(),
            finished: true,
            success,
        });
    }

    fn write(&self, state: ApplyProgress) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(&state)?)?;
        Ok(())
    }
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
