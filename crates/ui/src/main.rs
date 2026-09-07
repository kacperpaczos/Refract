pub mod dev_page;
pub mod history_page;
pub mod layout_page;
pub mod setup_page;
pub mod window;

use anyhow::Result;
use app::{history_service::HistoryService, snapshot_service::SnapshotService};
use app::layout_service::LayoutService;
use domain::{history::{ChangeKind, HistoryEntry}, snapshot::SnapshotKind};
use gtk4::{prelude::*, Application};
use simplelog::*;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

const APP_ID: &str = "org.desktop.ExperienceSwitcher";

fn main() -> Result<()> {
    std::fs::create_dir_all("logs").unwrap_or_default();

    let log_level = match std::env::var("RUST_LOG").as_deref().unwrap_or("info") {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    let log_config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .set_target_level(LevelFilter::Off)
        .set_thread_level(LevelFilter::Debug)
        .set_location_level(LevelFilter::Debug)
        .build();

    let _ = CombinedLogger::init(vec![
        TermLogger::new(log_level, log_config.clone(), TerminalMode::Stderr, ColorChoice::Auto),
        WriteLogger::new(
            log_level,
            log_config,
            OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/log.txt")
                .unwrap(),
        ),
    ]);

    if let Some(layout_id) = parse_apply_layout_arg() {
        log::info!("CLI mode: apply_layout layout_id={}", layout_id);
        return run_apply_layout_cli(&layout_id);
    }

    log::info!("┌─────────────────────────────────────────────────────┐");
    log::info!("│   Desktop Experience Switcher — starting up...      │");
    log::info!("└─────────────────────────────────────────────────────┘");
    log::info!("Data dir: {:?}", std::env::var("DESKTOP_EXPERIENCE_DATA_DIR").unwrap_or_else(|_| "(auto detect)".into()));
    log::info!("XDG_CURRENT_DESKTOP: {:?}", std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "(not set)".into()));

    // On-launch snapshot — nie blokuje, błąd logujemy i jedziemy dalej
    match perform_startup_snapshot() {
        Ok(_) => log::info!("Startup snapshot: OK"),
        Err(e) => log::warn!("Startup snapshot failed (non-fatal): {}", e),
    }

    log::info!("Building GTK4 window...");
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    let exit_code = app.run();
    log::info!("Application exited with: {:?}", exit_code);

    Ok(())
}

fn parse_apply_layout_arg() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--apply-layout" {
            return args.next();
        }
    }
    None
}

fn run_apply_layout_cli(layout_id: &str) -> Result<()> {
    let data_dir = resolve_data_dir();
    log::info!("run_apply_layout_cli(): data_dir={:?}", data_dir);
    let layouts = LayoutService::load_layouts(&data_dir)?;
    let layout = layouts
        .into_iter()
        .find(|layout| layout.id == layout_id)
        .ok_or_else(|| anyhow::anyhow!("Layout not found: {}", layout_id))?;

    let service = LayoutService::new()?;
    let report = service.apply_layout(&layout)?;
    if report.is_success() {
        log::info!("run_apply_layout_cli(): layout applied successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Layout apply finished with status {:?}: {}",
            report.status,
            report.detail
        ))
    }
}

pub(crate) fn resolve_data_dir() -> PathBuf {
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

fn build_ui(app: &Application) {
    log::info!("build_ui(): app_id={}", app.application_id().as_deref().unwrap_or("(missing)"));
    let window = window::build(app);
    log::info!("build_ui(): presenting main window");
    window.present();
}

fn perform_startup_snapshot() -> Result<()> {
    log::info!("perform_startup_snapshot(): begin");
    let snapshot_svc = SnapshotService::new()?;
    let history_svc = HistoryService::new()?;

    let snapshot = snapshot_svc.create_snapshot(SnapshotKind::OnLaunch)?;

    let entry = HistoryEntry::new(ChangeKind::AppLaunch, None, Some(snapshot.id));
    history_svc.log_change(entry)?;

    log::info!("perform_startup_snapshot(): done snapshot_id={}", snapshot.id);
    Ok(())
}
