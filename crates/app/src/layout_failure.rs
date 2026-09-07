use crate::history_service::HistoryService;
use crate::layout_tracker::LayoutTracker;
use crate::progress::ProgressReporter;
use crate::snapshot_service::SnapshotService;
use domain::history::{ChangeKind, HistoryEntry};
use domain::layout::{ApplyLayoutReport, ApplyStatus, LayoutDefinition, PreflightReport};
use infra::logging::log_err;
use std::collections::HashSet;
use std::time::Instant;
use uuid::Uuid;

pub struct LayoutFailureHandler<'a> {
    pub snapshot_service: &'a SnapshotService,
    pub history_service: &'a HistoryService,
    pub layout_tracker: &'a LayoutTracker,
}

pub struct FailureContext<'a> {
    pub layout: &'a LayoutDefinition,
    pub desktop_key: &'a str,
    pub started_at: Instant,
    pub progress: &'a ProgressReporter,
    pub snapshot_before: Option<Uuid>,
    pub installed_extensions: HashSet<String>,
    pub disabled_extensions: HashSet<String>,
    pub preflight: Option<PreflightReport>,
    pub plan_summary: Option<String>,
    pub applied_steps: usize,
    pub total_steps: usize,
    pub detail: String,
}

impl<'a> LayoutFailureHandler<'a> {
    pub fn finalize_tracker_state(
        &self,
        layout: &LayoutDefinition,
        desktop_key: &str,
        installed_extensions: HashSet<String>,
        disabled_extensions: HashSet<String>,
        snapshot_before: Option<Uuid>,
        snapshot_after: Option<Uuid>,
        preflight: Option<PreflightReport>,
        report: Option<ApplyLayoutReport>,
        plan_summary: Option<String>,
    ) {
        let _ = self.layout_tracker.save_deployment_state(
            &layout.id,
            desktop_key,
            layout.provider.clone(),
            layout.variant.clone(),
            installed_extensions,
            disabled_extensions,
            snapshot_before.map(|id| id.to_string()),
            snapshot_after.map(|id| id.to_string()),
            preflight,
            report,
            plan_summary,
        );
    }

    pub fn fail_with_optional_rollback(&self, ctx: FailureContext<'_>) -> ApplyLayoutReport {
        if let Some(snapshot_id) = ctx.snapshot_before {
            ctx.progress.update(ctx.applied_steps, "rollback", "Restoring snapshot after failed apply");
            match self.snapshot_service.restore_snapshot(&snapshot_id) {
                Ok(()) => {
                    let _ = self.history_service.log_change(HistoryEntry::new(
                        ChangeKind::SnapshotRestored(snapshot_id),
                        Some(snapshot_id),
                        None,
                    ));
                    let _ = self.history_service.log_change(HistoryEntry::new(
                        ChangeKind::LayoutRolledBack(ctx.layout.id.clone()),
                        Some(snapshot_id),
                        None,
                    ));
                    let report = ApplyLayoutReport {
                        layout_id: ctx.layout.id.clone(),
                        status: ApplyStatus::RolledBack,
                        applied_steps: ctx.applied_steps,
                        total_steps: ctx.total_steps,
                        detail: format!("{}\nRollback restored snapshot {}", ctx.detail, snapshot_id),
                    };
                    ctx.progress.finish(false, &report.detail);
                    self.finalize_tracker_state(
                        ctx.layout,
                        ctx.desktop_key,
                        ctx.installed_extensions,
                        ctx.disabled_extensions,
                        Some(snapshot_id),
                        None,
                        ctx.preflight,
                        Some(report.clone()),
                        ctx.plan_summary,
                    );
                    log_err("LayoutService::apply_layout", ctx.started_at, &anyhow::anyhow!("{}", report.detail));
                    return report;
                }
                Err(error) => {
                    let report = ApplyLayoutReport::failed_partial(
                        ctx.layout.id.clone(),
                        ctx.applied_steps,
                        ctx.total_steps,
                        format!("{}\nRollback failed for snapshot {}: {}", ctx.detail, snapshot_id, error),
                    );
                    ctx.progress.finish(false, &report.detail);
                    let _ = self.history_service.log_change(HistoryEntry::new(
                        ChangeKind::LayoutApplyFailed(ctx.layout.id.clone()),
                        Some(snapshot_id),
                        None,
                    ));
                    self.finalize_tracker_state(
                        ctx.layout,
                        ctx.desktop_key,
                        ctx.installed_extensions,
                        ctx.disabled_extensions,
                        Some(snapshot_id),
                        None,
                        ctx.preflight,
                        Some(report.clone()),
                        ctx.plan_summary,
                    );
                    log_err("LayoutService::apply_layout", ctx.started_at, &anyhow::anyhow!("{}", report.detail));
                    return report;
                }
            }
        }

        let report = ApplyLayoutReport::failed_partial(
            ctx.layout.id.clone(),
            ctx.applied_steps,
            ctx.total_steps,
            ctx.detail,
        );
        ctx.progress.finish(false, &report.detail);
        let _ = self.history_service.log_change(HistoryEntry::new(
            ChangeKind::LayoutApplyFailed(ctx.layout.id.clone()),
            None,
            None,
        ));
        self.finalize_tracker_state(
            ctx.layout,
            ctx.desktop_key,
            ctx.installed_extensions,
            ctx.disabled_extensions,
            None,
            None,
            ctx.preflight,
            Some(report.clone()),
            ctx.plan_summary,
        );
        log_err("LayoutService::apply_layout", ctx.started_at, &anyhow::anyhow!("{}", report.detail));
        report
    }
}
