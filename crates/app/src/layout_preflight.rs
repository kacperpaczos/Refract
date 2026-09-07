use anyhow::Result;
use crate::execution_plan::BackendCapabilityGraph;
use domain::layout::{LayoutDefinition, PreflightReport, StepType, VerifyType};
use infra::{executor, platform::PlatformInfo, tools};
use infra::tools::TOOL_GEXT;
use std::process::Command;

pub struct LayoutPreflightService;

impl LayoutPreflightService {
    pub fn detect_current_de() -> String {
        std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_else(|_| "gnome".to_string())
            .to_lowercase()
    }

    pub fn detect_de_key(current_de: &str) -> &'static str {
        if current_de.contains("gnome") {
            "gnome"
        } else if current_de.contains("kde") {
            "kde"
        } else {
            "gnome"
        }
    }

    pub fn run_verify_check(verify: &VerifyType) -> Result<bool> {
        match verify {
            VerifyType::Bash { command } => executor::verify_bash(command),
            VerifyType::GSettingCheck { schema, key, expected } => {
                executor::verify_gsetting(schema, key, expected)
            }
        }
    }

    pub fn preflight_layout(platform: &PlatformInfo, layout: &LayoutDefinition) -> Result<PreflightReport> {
        let current_de = Self::detect_current_de();
        let de_key = Self::detect_de_key(&current_de);
        let mut report = PreflightReport::new(layout.id.clone(), de_key);
        let capability_graph = BackendCapabilityGraph::for_backend(de_key);

        let Some(desktop_config) = layout.desktops.get(de_key) else {
            report.block(
                "desktop-config-missing",
                format!("Layout does not define configuration for desktop environment '{}'", de_key),
            );
            return Ok(report);
        };

        if let Some(min_version) = &desktop_config.min_version {
            match Self::detect_desktop_version(de_key) {
                Some(actual_version) => {
                    if !Self::version_matches_min(min_version, &actual_version) {
                        report.block(
                            "desktop-version-unsupported",
                            format!(
                                "Desktop version {} does not satisfy minimum required version {} for {}",
                                actual_version, min_version, de_key
                            ),
                        );
                    }
                }
                None => {
                    report.warn(
                        "desktop-version-unknown",
                        format!(
                            "Could not determine {} version; minimum required version is {}",
                            de_key, min_version
                        ),
                    );
                }
            }
        }

        for capability in &layout.capabilities_required {
            if !capability_graph.supports(capability.as_str()) {
                report.block(
                    "required-capability-unsupported",
                    format!(
                        "Required capability '{}' is not supported by backend '{}'",
                        capability, de_key
                    ),
                );
            }
        }

        for capability in &layout.capabilities_optional {
            if !capability_graph.supports(capability.as_str()) {
                report.warn(
                    "optional-capability-unsupported",
                    format!(
                        "Optional capability '{}' is not supported by backend '{}'",
                        capability, de_key
                    ),
                );
            }
        }

        for step in &desktop_config.steps {
            if let Some(os_list) = &step.if_os {
                if !platform.matches_os(os_list) {
                    report.warn(
                        "conditional-os-skip",
                        format!(
                            "Some steps are conditional and will be skipped because current OS '{}' is not in {:?}",
                            platform.os_id, os_list
                        ),
                    );
                }
            }

            if let Some(pm_list) = &step.if_pm {
                let current_pm = platform.package_manager.as_deref().unwrap_or("unknown");
                if !pm_list.iter().any(|pm| pm == current_pm) {
                    report.warn(
                        "conditional-pm-skip",
                        format!(
                            "Some steps are conditional and will be skipped because current package manager '{}' is not in {:?}",
                            current_pm, pm_list
                        ),
                    );
                }
            }

            match &step.action {
                StepType::ExtensionInstall { .. }
                | StepType::ExtensionEnable { .. }
                | StepType::ExtensionDisable { .. }
                | StepType::ExtensionUninstall { .. } => {
                    if !tools::is_installed(&TOOL_GEXT) {
                        report.warn(
                            "gext-missing",
                            "Tool 'gext' is not currently installed; the apply path may trigger interactive installation",
                        );
                    }
                }
                StepType::ExtensionInstallBundled { .. } => {
                    if !tools::command_exists("gsettings") {
                        report.block("gsettings-missing", "Bundled extension install requires 'gsettings'");
                    }
                }
                StepType::DconfLoad { .. } => {
                    if !tools::command_exists("dconf") {
                        report.block("dconf-missing", "Required command 'dconf' is not available");
                    }
                }
                StepType::GSettingSet { .. } => {
                    if !tools::command_exists("gsettings") {
                        report.block("gsettings-missing", "Required command 'gsettings' is not available");
                    }
                }
                StepType::GnomeShellReload { .. } => {
                    if de_key != "gnome" {
                        report.block(
                            "shell-reload-unsupported",
                            "GNOME Shell reload step is only supported on GNOME backends",
                        );
                    } else if !tools::command_exists("busctl") {
                        report.block("busctl-missing", "Required command 'busctl' is not available");
                    }
                }
                StepType::SystemPackageInstall { packages } => {
                    let Some(pm) = platform.package_manager.as_ref() else {
                        report.block(
                            "package-manager-unknown",
                            "Layout requires system package installation but the package manager could not be detected",
                        );
                        continue;
                    };
                    if !packages.contains_key(pm) {
                        report.block(
                            "package-list-missing",
                            format!("Layout requires package installation but provides no package list for '{}'", pm),
                        );
                    }
                }
                StepType::Bash { .. } => {
                    if !tools::command_exists("bash") {
                        report.block("bash-missing", "Required command 'bash' is not available");
                    }
                }
            }
        }

        for (index, check) in desktop_config.preflight_checks.iter().enumerate() {
            match Self::run_verify_check(check) {
                Ok(true) => {}
                Ok(false) => report.block(
                    "preflight-check-failed",
                    format!(
                        "Preflight check {}/{} failed",
                        index + 1,
                        desktop_config.preflight_checks.len()
                    ),
                ),
                Err(error) => report.block(
                    "preflight-check-error",
                    format!(
                        "Preflight check {}/{} errored: {}",
                        index + 1,
                        desktop_config.preflight_checks.len(),
                        error
                    ),
                ),
            }
        }

        Ok(report)
    }

    fn detect_desktop_version(de_key: &str) -> Option<String> {
        let output = match de_key {
            "gnome" => Command::new("gnome-shell").arg("--version").output().ok()?,
            "kde" => Command::new("plasmashell").arg("--version").output().ok()?,
            _ => return None,
        };

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{} {}", stdout, stderr);

        combined
            .split_whitespace()
            .find_map(|token| {
                let normalized = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.');
                if normalized.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    Some(normalized.to_string())
                } else {
                    None
                }
            })
    }

    fn version_matches_min(min_version: &str, actual_version: &str) -> bool {
        let parse_major = |value: &str| -> Option<u32> {
            value.split('.').next()?.parse::<u32>().ok()
        };

        match (parse_major(min_version), parse_major(actual_version)) {
            (Some(min), Some(actual)) => actual >= min,
            _ => false,
        }
    }
}
