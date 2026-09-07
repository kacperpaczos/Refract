use domain::layout::{DesktopConfig, LayoutDefinition, Step};
use infra::platform::PlatformInfo;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityLevel {
    Native,
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct BackendCapabilityGraph {
    pub backend_key: String,
    pub capabilities: HashMap<String, CapabilityLevel>,
}

impl BackendCapabilityGraph {
    pub fn for_backend(desktop_key: &str) -> Self {
        let mut capabilities = HashMap::from([
            ("gsettings".to_string(), CapabilityLevel::Native),
            ("dconf".to_string(), CapabilityLevel::Native),
            ("extensions.install".to_string(), CapabilityLevel::Native),
            ("extensions.enable".to_string(), CapabilityLevel::Native),
            ("extensions.disable".to_string(), CapabilityLevel::Native),
            ("extensions.uninstall".to_string(), CapabilityLevel::Native),
            ("bash".to_string(), CapabilityLevel::Native),
            ("system.package_install".to_string(), CapabilityLevel::Native),
            ("health_checks".to_string(), CapabilityLevel::Native),
            ("preflight_checks".to_string(), CapabilityLevel::Native),
        ]);

        if desktop_key == "gnome" {
            capabilities.insert("gnome.shell_reload".to_string(), CapabilityLevel::Native);
        } else {
            capabilities.insert("gnome.shell_reload".to_string(), CapabilityLevel::Unsupported);
        }

        Self {
            backend_key: desktop_key.to_string(),
            capabilities,
        }
    }

    pub fn supports(&self, capability: &str) -> bool {
        matches!(
            self.capabilities.get(capability),
            Some(CapabilityLevel::Native)
        )
    }
}

#[derive(Debug, Clone)]
pub struct PlannedStep {
    pub original_index: usize,
    pub step: Step,
}

#[derive(Debug, Clone)]
pub struct SkippedStep {
    pub original_index: usize,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub layout_id: String,
    pub desktop_key: String,
    pub total_defined_steps: usize,
    pub executable_steps: Vec<PlannedStep>,
    pub skipped_steps: Vec<SkippedStep>,
    pub post_flight_wait_ms: u64,
    pub capability_graph: BackendCapabilityGraph,
}

impl ExecutionPlan {
    pub fn summary(&self) -> String {
        format!(
            "Plan for '{}' on '{}': {} executable, {} skipped, {} defined",
            self.layout_id,
            self.desktop_key,
            self.executable_steps.len(),
            self.skipped_steps.len(),
            self.total_defined_steps
        )
    }
}

pub struct ExecutionPlanner;

impl ExecutionPlanner {
    pub fn build(
        layout: &LayoutDefinition,
        desktop_key: &str,
        desktop_config: &DesktopConfig,
        platform: &PlatformInfo,
    ) -> ExecutionPlan {
        let capability_graph = BackendCapabilityGraph::for_backend(desktop_key);
        let mut executable_steps = Vec::new();
        let mut skipped_steps = Vec::new();

        for (index, step) in desktop_config.steps.iter().enumerate() {
            if let Some(os_list) = &step.if_os {
                if !platform.matches_os(os_list) {
                    skipped_steps.push(SkippedStep {
                        original_index: index,
                        reason: format!("OS mismatch: current '{}' not in {:?}", platform.os_id, os_list),
                    });
                    continue;
                }
            }

            if let Some(pm_list) = &step.if_pm {
                let current_pm = platform.package_manager.as_deref().unwrap_or("unknown");
                if !pm_list.iter().any(|pm| pm == current_pm) {
                    skipped_steps.push(SkippedStep {
                        original_index: index,
                        reason: format!(
                            "Package manager mismatch: current '{}' not in {:?}",
                            current_pm, pm_list
                        ),
                    });
                    continue;
                }
            }

            executable_steps.push(PlannedStep {
                original_index: index,
                step: step.clone(),
            });
        }

        ExecutionPlan {
            layout_id: layout.id.clone(),
            desktop_key: desktop_key.to_string(),
            total_defined_steps: desktop_config.steps.len(),
            executable_steps,
            skipped_steps,
            post_flight_wait_ms: desktop_config.post_flight_wait_ms,
            capability_graph,
        }
    }
}
