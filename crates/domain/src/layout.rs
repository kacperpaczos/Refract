use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutDefinition {
    pub id: String,
    pub label: String,
    pub preview: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_spec_version")]
    pub spec_version: u32,
    pub provider: Option<String>,
    pub variant: Option<String>,
    #[serde(default)]
    pub capabilities_required: Vec<String>,
    #[serde(default)]
    pub capabilities_optional: Vec<String>,
    pub degraded_behavior: Option<String>,
    pub rollback_policy: Option<String>,
    
    #[serde(default)]
    pub desktops: std::collections::HashMap<String, DesktopConfig>,
}

fn default_spec_version() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApplyStatus {
    Applied,
    Degraded,
    Blocked,
    RolledBack,
    FailedPartial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyLayoutReport {
    pub layout_id: String,
    pub status: ApplyStatus,
    pub applied_steps: usize,
    pub total_steps: usize,
    pub detail: String,
}

impl ApplyLayoutReport {
    pub fn applied(layout_id: impl Into<String>, applied_steps: usize, total_steps: usize, detail: impl Into<String>) -> Self {
        Self {
            layout_id: layout_id.into(),
            status: ApplyStatus::Applied,
            applied_steps,
            total_steps,
            detail: detail.into(),
        }
    }

    pub fn failed_partial(layout_id: impl Into<String>, applied_steps: usize, total_steps: usize, detail: impl Into<String>) -> Self {
        Self {
            layout_id: layout_id.into(),
            status: ApplyStatus::FailedPartial,
            applied_steps,
            total_steps,
            detail: detail.into(),
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, ApplyStatus::Applied)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreflightSeverity {
    Warning,
    Blocking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightIssue {
    pub severity: PreflightSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    pub layout_id: String,
    pub desktop: String,
    pub supported: bool,
    pub degraded: bool,
    pub issues: Vec<PreflightIssue>,
}

impl PreflightReport {
    pub fn new(layout_id: impl Into<String>, desktop: impl Into<String>) -> Self {
        Self {
            layout_id: layout_id.into(),
            desktop: desktop.into(),
            supported: true,
            degraded: false,
            issues: Vec::new(),
        }
    }

    pub fn warn(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.degraded = true;
        self.issues.push(PreflightIssue {
            severity: PreflightSeverity::Warning,
            code: code.into(),
            message: message.into(),
        });
    }

    pub fn block(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.supported = false;
        self.issues.push(PreflightIssue {
            severity: PreflightSeverity::Blocking,
            code: code.into(),
            message: message.into(),
        });
    }

    pub fn summary(&self) -> String {
        if self.issues.is_empty() {
            return "Preflight passed without issues".to_string();
        }

        self.issues
            .iter()
            .map(|issue| format!("[{:?}] {}: {}", issue.severity, issue.code, issue.message))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    pub min_version: Option<String>,
    #[serde(default = "default_post_flight_wait")]
    pub post_flight_wait_ms: u64,
    #[serde(default)]
    pub preflight_checks: Vec<VerifyType>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub health_checks: Vec<VerifyType>,
}

fn default_post_flight_wait() -> u64 { 1500 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StepType {
    #[serde(rename = "gsetting")]
    GSettingSet { schema: String, key: String, value: String },
    
    #[serde(rename = "dconf")]
    DconfLoad { path: String, action: String, data: String },
    
    #[serde(rename = "extension.enable")]
    ExtensionEnable { id: String },
    
    #[serde(rename = "extension.disable")]
    ExtensionDisable { id: String },

    #[serde(rename = "extension.install")]
    ExtensionInstall { id: String },

    #[serde(rename = "extension.install_bundled")]
    ExtensionInstallBundled { id: String, source: String },
    
    #[serde(rename = "extension.uninstall")]
    ExtensionUninstall { id: String },
    
    #[serde(rename = "bash")]
    Bash { command: String },

    /// Restartuje GNOME Shell (ładuje rozszerzenia i ich schematy gsettings).
    /// Wymagany po extension.install zanim gsettings dla wtyczki zadziała.
    #[serde(rename = "gnome.shell_reload")]
    GnomeShellReload {
        #[serde(default = "default_wait_ms")]
        wait_ms: u64,
    },

    #[serde(rename = "system.package_install")]
    SystemPackageInstall {
        #[serde(default)]
        packages: std::collections::HashMap<String, Vec<String>>,
    },
}

fn default_wait_ms() -> u64 { 3000 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VerifyType {
    #[serde(rename = "bash")]
    Bash { command: String },
    
    #[serde(rename = "gsetting_check")]
    GSettingCheck { schema: String, key: String, expected: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    #[serde(flatten)]
    pub action: StepType,
    pub verify: Option<VerifyType>,
    pub if_os: Option<Vec<String>>,
    pub if_pm: Option<Vec<String>>,
}
