use anyhow::Result;
use crate::execution_plan::ExecutionPlanner;
use crate::layout_failure::{FailureContext, LayoutFailureHandler};
use crate::layout_health::LayoutHealthService;
use crate::layout_preflight::LayoutPreflightService;
use domain::history::{ChangeKind, HistoryEntry};
use domain::layout::{ApplyLayoutReport, ApplyStatus, LayoutDefinition, PreflightReport, StepType, VerifyType};
use domain::snapshot::SnapshotKind;
use infra::{dconf, executor, extensions, gsettings, platform::PlatformInfo};
use infra::logging::{log_enter, log_err, log_ok, preview_debug, preview_lines, preview_str};
use std::fs;
use std::path::{Path, PathBuf};

use crate::history_service::HistoryService;
use crate::layout_tracker::LayoutTracker;
use crate::progress::ProgressReporter;
use crate::snapshot_service::SnapshotService;
use std::collections::HashSet;

pub struct LayoutService {
    snapshot_service: SnapshotService,
    history_service: HistoryService,
    layout_tracker: LayoutTracker,
    platform: PlatformInfo,
}

impl LayoutService {
    pub fn new() -> Result<Self> {
        let started_at = log_enter("LayoutService::new", "");
        let platform = PlatformInfo::current().unwrap_or_else(|e| {
            log::warn!("Failed to detect platform: {}", e);
            PlatformInfo {
                os_id: String::new(),
                os_like: Vec::new(),
                package_manager: None,
            }
        });

        let snapshot_service = SnapshotService::new()?;
        let history_service = HistoryService::new()?;
        let layout_tracker = LayoutTracker::new()?;

        let service = Self {
            snapshot_service,
            history_service,
            layout_tracker,
            platform,
        };
        log_ok(
            "LayoutService::new",
            started_at,
            &format!(
                "platform.os_id={} package_manager={:?}",
                service.platform.os_id,
                service.platform.package_manager
            ),
        );
        Ok(service)
    }

    pub fn load_layouts(data_dir: &Path) -> Result<Vec<LayoutDefinition>> {
        let started_at = log_enter("LayoutService::load_layouts", &format!("data_dir={:?}", data_dir));
        let mut defs = Vec::new();
        let mut layout_dirs = vec![data_dir.join("layouts")];
        if let Some(mut user_dir) = dirs::data_local_dir() {
            user_dir.push("desktop-experience-switcher");
            user_dir.push("layouts");
            layout_dirs.push(user_dir);
        }

        for layouts_dir in layout_dirs {
            if !layouts_dir.exists() {
                log::info!("LayoutService::load_layouts: skipping missing dir {:?}", layouts_dir);
                continue;
            }

            for entry in fs::read_dir(&layouts_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "de") {
                    let content = fs::read_to_string(&path)?;
                    log::info!(
                        "LayoutService::load_layouts: reading file={:?} bytes={} preview={}",
                        path,
                        content.len(),
                        preview_lines(&content, 4)
                    );
                    match serde_yaml::from_str::<LayoutDefinition>(&content) {
                        Ok(def) => defs.push(def),
                        Err(e) => log::error!("Failed to parse layout {:?}: {}", path, e),
                    }
                }
            }
        }
        log_ok("LayoutService::load_layouts", started_at, &format!("count={}", defs.len()));
        Ok(defs)
    }

    pub fn preflight_layout(&self, layout: &LayoutDefinition) -> Result<PreflightReport> {
        LayoutPreflightService::preflight_layout(&self.platform, layout)
    }

    pub fn apply_layout(&self, layout: &LayoutDefinition) -> Result<ApplyLayoutReport> {
        let started_at = log_enter(
            "LayoutService::apply_layout",
            &format!("layout_id={} label={} desktops={}", layout.id, layout.label, preview_debug(&layout.desktops.keys().collect::<Vec<_>>())),
        );
        log::info!("Applying layout: {} ({})", layout.label, layout.id);

        let current_de = LayoutPreflightService::detect_current_de();
        let de_key = LayoutPreflightService::detect_de_key(&current_de);

        let preflight = self.preflight_layout(layout)?;
        if !preflight.supported {
            let detail = format!("Preflight blocked layout apply:\n{}", preflight.summary());
            progress_blocked(layout, &detail);
            let report = ApplyLayoutReport {
                layout_id: layout.id.clone(),
                status: ApplyStatus::Blocked,
                applied_steps: 0,
                total_steps: layout
                    .desktops
                    .get(de_key)
                    .map(|cfg| cfg.steps.len())
                    .unwrap_or(0),
                detail,
            };
            let _ = self.history_service.log_change(HistoryEntry::new(
                ChangeKind::LayoutApplyBlocked(layout.id.clone()),
                None,
                None,
            ));
            LayoutFailureHandler {
                snapshot_service: &self.snapshot_service,
                history_service: &self.history_service,
                layout_tracker: &self.layout_tracker,
            }
            .finalize_tracker_state(
                layout,
                de_key,
                HashSet::new(),
                HashSet::new(),
                None,
                None,
                Some(preflight.clone()),
                Some(report.clone()),
                None,
            );
            log_err(
                "LayoutService::apply_layout",
                started_at,
                &anyhow::anyhow!("{}", report.detail),
            );
            return Ok(report);
        }
        
        let desktop_config = layout.desktops.get(de_key).ok_or_else(|| {
            anyhow::anyhow!("No configuration found for desktop environment: {}", de_key)
        })?;
        let execution_plan = ExecutionPlanner::build(layout, de_key, desktop_config, &self.platform);
        let plan_summary = execution_plan.summary();
        log::info!(
            "LayoutService::apply_layout: selected desktop_config key={} defined_steps={} executable_steps={} skipped_steps={} post_flight_wait_ms={}",
            de_key,
            execution_plan.total_defined_steps,
            execution_plan.executable_steps.len(),
            execution_plan.skipped_steps.len(),
            execution_plan.post_flight_wait_ms
        );
        let progress = ProgressReporter::from_env(&layout.id, execution_plan.executable_steps.len());
        progress.update(0, "starting", "Preparing snapshot and reading current state");
        progress.update(0, "planning", &plan_summary);
        if preflight.degraded {
            progress.update(0, "preflight-warning", &preflight.summary());
        }
        if !execution_plan.skipped_steps.is_empty() {
            log::info!(
                "LayoutService::apply_layout: skipped_steps={}",
                preview_debug(
                    &execution_plan
                        .skipped_steps
                        .iter()
                        .map(|step| format!("#{} {}", step.original_index + 1, step.reason))
                        .collect::<Vec<_>>()
                )
            );
        }

        let snapshot_before = self
            .snapshot_service
            .create_snapshot(SnapshotKind::BeforeChange)
            .ok();
        log::info!(
            "LayoutService::apply_layout: snapshot_before={}",
            snapshot_before
                .as_ref()
                .map(|s| s.id.to_string())
                .unwrap_or_else(|| "(none)".to_string())
        );

        let previously_installed = self.layout_tracker.get_installed_extensions();
        let mut newly_required = HashSet::new();
        let mut disabled_extensions = HashSet::new();

        // Zebranie wymaganych wtyczek przez nowy layout
        for planned_step in &execution_plan.executable_steps {
            let step = &planned_step.step;
            match &step.action {
                StepType::ExtensionInstall { id } | StepType::ExtensionInstallBundled { id, .. } => {
                    newly_required.insert(id.clone());
                }
                StepType::ExtensionDisable { id } => {
                    disabled_extensions.insert(id.clone());
                }
                _ => {}
            }
        }
        log::info!(
            "LayoutService::apply_layout: previously_installed={} newly_required={}",
            preview_debug(&previously_installed),
            preview_debug(&newly_required)
        );

        let to_uninstall: HashSet<_> = previously_installed.difference(&newly_required).collect();
        if !to_uninstall.is_empty() {
            log::info!("Auto-Uninstall: Usuwam {} nieużywanych już rozszerzeń...", to_uninstall.len());
            for id in to_uninstall {
                let _ = extensions::uninstall_extension(id);
            }
        }

        let total_steps = execution_plan.executable_steps.len();

        for (i, planned_step) in execution_plan.executable_steps.iter().enumerate() {
            let step = &planned_step.step;
            progress.update(
                i,
                "running",
                &format!("Starting step {}/{}: {:?}", i + 1, total_steps, step.action),
            );
            log::info!(
                "LayoutService::apply_layout: step_index={} total_steps={} original_index={} action={}",
                i + 1,
                total_steps,
                planned_step.original_index + 1,
                preview_debug(&step.action)
            );

            let exec_result = match &step.action {
                StepType::GSettingSet { schema, key, value } => {
                    log::info!("Step {} action=GSettingSet schema={} key={} value={}", i + 1, schema, key, preview_str(value));
                    gsettings::set(schema, key, value)
                }
                StepType::DconfLoad { path, data, .. } => {
                    log::info!("Step {} action=DconfLoad path={} data_preview={}", i + 1, path, preview_lines(data, 3));
                    dconf::load(path, data)
                }
                StepType::ExtensionEnable { id } => {
                    log::info!("Step {} action=ExtensionEnable id={}", i + 1, id);
                    extensions::enable_extension(id).map(|_| ())
                }
                StepType::ExtensionDisable { id } => {
                    log::info!("Step {} action=ExtensionDisable id={}", i + 1, id);
                    extensions::disable_extension(id).map(|_| ())
                }
                StepType::ExtensionInstall { id } => {
                    log::info!("Step {} action=ExtensionInstall id={}", i + 1, id);
                    progress.update_stage(
                        i + 1,
                        "extension.download",
                        "Downloading extension",
                        Some(id),
                        &format!("Starting download for {}", id),
                    );
                    let res = extensions::install_extension_with_progress(id, |phase, detail| {
                        progress.update_stage(i + 1, phase, "Installing extension", Some(id), detail);
                    })
                    .map(|_| ());
                    if res.is_ok() {
                        progress.update_stage(
                            i + 1,
                            "extension.register",
                            "Refreshing GNOME extension registry",
                            Some(id),
                            &format!("Registering {} in the current GNOME session", id),
                        );
                        log::info!("Reloading GNOME Shell extensions list...");
                        let _ = executor::execute_bash(
                            "busctl --user call org.gnome.Shell /org/gnome/Shell \
                             org.gnome.Shell Eval s \
                             'ExtensionSystem.scanForUpdates()' 2>/dev/null || true"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    }
                    res
                }
                StepType::ExtensionInstallBundled { id, source } => {
                    log::info!(
                        "Step {} action=ExtensionInstallBundled id={} source={}",
                        i + 1,
                        id,
                        source
                    );
                    let source_dir = resolve_runtime_data_dir().join(source);
                    progress.update_stage(
                        i + 1,
                        "extension.bundle-prepare",
                        "Preparing bundled extension",
                        Some(id),
                        &format!("Using bundled extension files from {:?}", source_dir),
                    );
                    let res = extensions::install_bundled_extension_with_progress(id, &source_dir, |phase, detail| {
                        progress.update_stage(i + 1, phase, "Installing bundled extension", Some(id), detail);
                    })
                    .map(|_| ());
                    if res.is_ok() {
                        progress.update_stage(
                            i + 1,
                            "extension.register",
                            "Refreshing GNOME extension registry",
                            Some(id),
                            &format!("Registering bundled extension {} in GNOME Shell", id),
                        );
                        let _ = executor::execute_bash(
                            "busctl --user call org.gnome.Shell /org/gnome/Shell \
                             org.gnome.Shell Eval s \
                             'ExtensionSystem.scanForUpdates()' 2>/dev/null || true"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    }
                    res
                }
                StepType::ExtensionUninstall { id } => {
                    log::info!("Step {} action=ExtensionUninstall id={}", i + 1, id);
                    extensions::uninstall_extension(id).map(|_| ())
                }
                StepType::GnomeShellReload { wait_ms } => {
                    log::info!("Przeładowanie GNOME Shell (czekam {}ms na gotowość schematów)...", wait_ms);
                    // Wyślij sygnał restartu przez D-Bus (bezpieczny na Wayland GNOME 45+)
                    let _ = executor::execute_bash(
                        "busctl --user call org.gnome.Shell /org/gnome/Shell \
                         org.gnome.Shell Eval s 'Meta.restart(\"Reloading extensions...\")' \
                         2>/dev/null || true"
                    );
                    // Czekamy aż GNOME Shell wróci do życia i załaduje schematy
                    std::thread::sleep(std::time::Duration::from_millis(*wait_ms));
                    log::info!("GNOME Shell przeładowany — schematy gsettings powinny być dostępne.");
                    Ok(())
                }
                StepType::Bash { command } => {
                    log::info!("Step {} action=Bash command={}", i + 1, preview_str(command));
                    match executor::execute_bash(command) {
                        Ok(stdout) => {
                            if !stdout.trim().is_empty() {
                                log::debug!("Bash stdout: {}", stdout.trim());
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                StepType::SystemPackageInstall { packages } => {
                    log::info!("Step {} action=SystemPackageInstall packages={}", i + 1, preview_debug(packages));
                    let pm = self.platform.package_manager.as_ref();
                    if let Some(manager) = pm {
                        if let Some(pkgs) = packages.get(manager) {
                            if !pkgs.is_empty() {
                                let pkg_list = pkgs.join(" ");
                                let cmd = match manager.as_str() {
                                    "apt" => format!("pkexec apt-get install -y {}", pkg_list),
                                    "dnf" => format!("pkexec dnf install -y {}", pkg_list),
                                    "pacman" => format!("pkexec pacman -S --noconfirm {}", pkg_list),
                                    "zypper" => format!("pkexec zypper install -y {}", pkg_list),
                                    _ => String::new(),
                                };
                                
                                if !cmd.is_empty() {
                                    log::info!("Installing packages via {}: {}", manager, pkg_list);
                                    let _ = executor::execute_bash(&cmd)?;
                                }
                            }
                            Ok(())
                        } else {
                            log::warn!("No package list provided for package manager: {}", manager);
                            Ok(())
                        }
                    } else {
                        log::warn!("Cannot install packages: Package manager unknown for this OS");
                        Ok(())
                    }
                }
            };

            if let Err(e) = exec_result {
                log::error!("Step {} execution failed: {}", i + 1, e);
                progress.update(
                    i + 1,
                    "step-error",
                    &format!("Step {}/{} failed: {}", i + 1, total_steps, e),
                );
                let report = LayoutFailureHandler {
                    snapshot_service: &self.snapshot_service,
                    history_service: &self.history_service,
                    layout_tracker: &self.layout_tracker,
                }
                .fail_with_optional_rollback(FailureContext {
                    layout,
                    desktop_key: de_key,
                    started_at,
                    progress: &progress,
                    snapshot_before: snapshot_before.as_ref().map(|s| s.id),
                    installed_extensions: newly_required.clone(),
                    disabled_extensions: disabled_extensions.clone(),
                    preflight: Some(preflight.clone()),
                    plan_summary: Some(plan_summary.clone()),
                    applied_steps: i,
                    total_steps,
                    detail: format!("Execution failed on step {}/{}: {}", i + 1, total_steps, e),
                });
                return Ok(report);
            } else {
                log::info!("LayoutService::apply_layout: step {} execution completed", i + 1);
                progress.update(
                    i + 1,
                    "step-done",
                    &format!("Completed step {}/{}", i + 1, total_steps),
                );
            }

            if let Some(verify) = &step.verify {
                progress.update(
                    i + 1,
                    "verifying",
                    &format!("Verifying step {}/{}", i + 1, total_steps),
                );
                log::info!("LayoutService::apply_layout: step {} verification={}", i + 1, preview_debug(verify));
                let verify_result = match verify {
                    VerifyType::Bash { command } => executor::verify_bash(command),
                    VerifyType::GSettingCheck { schema, key, expected } => {
                        executor::verify_gsetting(schema, key, expected)
                    }
                };

                match verify_result {
                    Ok(true) => log::info!("Step {} verified successfully", i + 1),
                    Ok(false) => {
                        let detail = format!("Verification failed on step {}/{} (state mismatch)", i + 1, total_steps);
                        log::warn!("Step {} verification failed (mismatch)", i + 1);
                        progress.update(i + 1, "verify-error", &detail);
                        let report = LayoutFailureHandler {
                            snapshot_service: &self.snapshot_service,
                            history_service: &self.history_service,
                            layout_tracker: &self.layout_tracker,
                        }
                        .fail_with_optional_rollback(FailureContext {
                            layout,
                            desktop_key: de_key,
                            started_at,
                            progress: &progress,
                            snapshot_before: snapshot_before.as_ref().map(|s| s.id),
                            installed_extensions: newly_required.clone(),
                            disabled_extensions: disabled_extensions.clone(),
                            preflight: Some(preflight.clone()),
                            plan_summary: Some(plan_summary.clone()),
                            applied_steps: i + 1,
                            total_steps,
                            detail,
                        });
                        return Ok(report);
                    }
                    Err(e) => {
                        let detail = format!("Verification failed on step {}/{}: {}", i + 1, total_steps, e);
                        log::error!("Step {} verification error: {}", i + 1, e);
                        progress.update(i + 1, "verify-error", &detail);
                        let report = LayoutFailureHandler {
                            snapshot_service: &self.snapshot_service,
                            history_service: &self.history_service,
                            layout_tracker: &self.layout_tracker,
                        }
                        .fail_with_optional_rollback(FailureContext {
                            layout,
                            desktop_key: de_key,
                            started_at,
                            progress: &progress,
                            snapshot_before: snapshot_before.as_ref().map(|s| s.id),
                            installed_extensions: newly_required.clone(),
                            disabled_extensions: disabled_extensions.clone(),
                            preflight: Some(preflight.clone()),
                            plan_summary: Some(plan_summary.clone()),
                            applied_steps: i + 1,
                            total_steps,
                            detail,
                        });
                        return Ok(report);
                    }
                }
            }
        }

        progress.update(
            desktop_config.steps.len(),
            "post-flight",
            "Checking final extension state and saving history",
        );
        log::info!("Weryfikacja post-flight stanu rozszerzeń GNOME...");
        
        // Zbieramy oczekiwania co do stanu rozszerzeń z widoku pliku .de
        let mut expected_enabled = std::collections::HashSet::new();
        let mut expected_disabled = std::collections::HashSet::new();

        for planned_step in &execution_plan.executable_steps {
            let step = &planned_step.step;
            match &step.action {
                StepType::ExtensionInstall { id }
                | StepType::ExtensionInstallBundled { id, .. }
                | StepType::ExtensionEnable { id } => {
                    expected_enabled.insert(id.clone());
                    expected_disabled.remove(id);
                }
                StepType::ExtensionUninstall { id } | StepType::ExtensionDisable { id } => {
                    expected_disabled.insert(id.clone());
                    expected_enabled.remove(id);
                }
                _ => {}
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(execution_plan.post_flight_wait_ms));
        let currently_enabled: std::collections::HashSet<String> = extensions::list_enabled().into_iter().collect();
        log::info!(
            "LayoutService::apply_layout: post_flight expected_enabled={} expected_disabled={} currently_enabled={}",
            preview_debug(&expected_enabled),
            preview_debug(&expected_disabled),
            preview_debug(&currently_enabled)
        );

        let mut verification_errors = Vec::new();

        for id in &expected_enabled {
            if !currently_enabled.contains(id) {
                verification_errors.push(format!("Rozszerzenie nie jest aktywne/gotowe: {}", id));
            }
        }

        for id in &expected_disabled {
            if currently_enabled.contains(id) {
                verification_errors.push(format!("Rozszerzenie powinno być wyłączone, a obsługuje sesję: {}", id));
            }
        }

        if !verification_errors.is_empty() {
            let error_msg = verification_errors.join("\n");
            log::warn!("Weryfikacja zakończona błędami:\n{}", error_msg);
            let error = anyhow::anyhow!(
                "GNOME nie włączył wtyczek poprawnie.\nUpewnij się, że są one kompatybilne z wersją GNOME.\nRozpocznij proces odświeżania powłoki.\n\nBłędy:\n{}", 
                error_msg
            );
            let report = LayoutFailureHandler {
                snapshot_service: &self.snapshot_service,
                history_service: &self.history_service,
                layout_tracker: &self.layout_tracker,
            }
            .fail_with_optional_rollback(FailureContext {
                layout,
                desktop_key: de_key,
                started_at,
                progress: &progress,
                snapshot_before: snapshot_before.as_ref().map(|s| s.id),
                installed_extensions: newly_required.clone(),
                disabled_extensions: disabled_extensions.clone(),
                preflight: Some(preflight.clone()),
                plan_summary: Some(plan_summary.clone()),
                applied_steps: total_steps,
                total_steps,
                detail: error.to_string(),
            });
            return Ok(report);
        }

        if !desktop_config.health_checks.is_empty() {
            progress.update(
                total_steps,
                "health-checks",
                &format!("Running {} layout health checks", desktop_config.health_checks.len()),
            );

            if let Some(detail) = LayoutHealthService::run_health_checks(&desktop_config.health_checks)? {
                progress.update(total_steps, "health-check-error", &detail);
                let report = LayoutFailureHandler {
                    snapshot_service: &self.snapshot_service,
                    history_service: &self.history_service,
                    layout_tracker: &self.layout_tracker,
                }
                .fail_with_optional_rollback(FailureContext {
                    layout,
                    desktop_key: de_key,
                    started_at,
                    progress: &progress,
                    snapshot_before: snapshot_before.as_ref().map(|s| s.id),
                    installed_extensions: newly_required.clone(),
                    disabled_extensions: disabled_extensions.clone(),
                    preflight: Some(preflight.clone()),
                    plan_summary: Some(plan_summary.clone()),
                    applied_steps: total_steps,
                    total_steps,
                    detail,
                });
                return Ok(report);
            }
        }

        let snapshot_after = self
            .snapshot_service
            .create_snapshot(SnapshotKind::AfterChange)
            .ok();
        log::info!(
            "LayoutService::apply_layout: snapshot_after={}",
            snapshot_after
                .as_ref()
                .map(|s| s.id.to_string())
                .unwrap_or_else(|| "(none)".to_string())
        );

        let entry = HistoryEntry::new(
            ChangeKind::LayoutApplied(layout.id.clone()),
            snapshot_before.as_ref().map(|s| s.id),
            snapshot_after.as_ref().map(|s| s.id),
        );
        self.history_service.log_change(entry)?;

        log::info!("Layout applied and verified successfully!");
        progress.finish(true, "Layout applied successfully");
        let report = if preflight.degraded {
            ApplyLayoutReport {
                layout_id: layout.id.clone(),
                status: ApplyStatus::Degraded,
                applied_steps: total_steps,
                total_steps,
                detail: format!("Layout applied with warnings:\n{}", preflight.summary()),
            }
        } else {
            ApplyLayoutReport::applied(
                layout.id.clone(),
                total_steps,
                total_steps,
                "Layout applied successfully",
            )
        };
        LayoutFailureHandler {
            snapshot_service: &self.snapshot_service,
            history_service: &self.history_service,
            layout_tracker: &self.layout_tracker,
        }
        .finalize_tracker_state(
            layout,
            de_key,
            newly_required,
            disabled_extensions,
            snapshot_before.as_ref().map(|s| s.id),
            snapshot_after.as_ref().map(|s| s.id),
            Some(preflight),
            Some(report.clone()),
            Some(plan_summary),
        );
        log_ok(
            "LayoutService::apply_layout",
            started_at,
            &format!("layout_id={} result={:?}", layout.id, report.status),
        );
        Ok(report)
    }
}

fn resolve_runtime_data_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("DESKTOP_EXPERIENCE_DATA_DIR") {
        PathBuf::from(env_dir)
    } else if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe.parent().unwrap_or(Path::new("."));
        let candidate = exe_dir.join("data");
        if candidate.exists() {
            candidate
        } else {
            PathBuf::from("/home/kacper/Desktop/gnome-sriczer/desktop-experience-switcher/data")
        }
    } else {
        PathBuf::from("/home/kacper/Desktop/gnome-sriczer/desktop-experience-switcher/data")
    }
}

fn progress_blocked(layout: &LayoutDefinition, detail: &str) {
    let total_steps = layout
        .desktops
        .values()
        .next()
        .map(|cfg| cfg.steps.len())
        .unwrap_or(0);
    let progress = ProgressReporter::from_env(&layout.id, total_steps);
    progress.update(0, "preflight-blocked", detail);
    progress.finish(false, detail);
}
