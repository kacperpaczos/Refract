use anyhow::{Context, Result};
use crate::layout_tracker::LayoutTracker;
use infra::{dconf, extensions, platform::PlatformInfo, tools};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct GnomeAdapterSnapshotService;

pub struct AdapterSnapshotResult {
    pub output_file: PathBuf,
}

#[derive(Serialize)]
struct GnomeAdapterSnapshot {
    meta: SnapshotMeta,
    platform: PlatformSnapshot,
    session: SessionSnapshot,
    tools: ToolsSnapshot,
    extensions: ExtensionsSnapshot,
    dconf: DconfSnapshot,
    layout_engine: LayoutEngineSnapshot,
}

#[derive(Serialize)]
struct SnapshotMeta {
    snapshot_type: String,
    created_at_unix: u64,
    format_version: u32,
}

#[derive(Serialize)]
struct PlatformSnapshot {
    os_id: String,
    os_like: Vec<String>,
    package_manager: Option<String>,
}

#[derive(Serialize)]
struct SessionSnapshot {
    desktop: String,
    desktop_key: String,
    session_type: Option<String>,
    gnome_shell_version: Option<String>,
}

#[derive(Serialize)]
struct ToolsSnapshot {
    gsettings: bool,
    dconf: bool,
    busctl: bool,
    gext: bool,
}

#[derive(Serialize)]
struct ExtensionsSnapshot {
    enabled: Vec<String>,
    installed: Vec<InstalledExtensionSnapshot>,
    dconf_sections_by_id: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct InstalledExtensionSnapshot {
    id: String,
    slug: String,
    enabled: bool,
    source: String,
    path: String,
    metadata: Option<Value>,
}

#[derive(Serialize)]
struct DconfSnapshot {
    org_gnome_dump: String,
}

#[derive(Serialize)]
struct LayoutEngineSnapshot {
    tracker_state: Option<Value>,
}

impl GnomeAdapterSnapshotService {
    pub fn create_snapshot() -> Result<AdapterSnapshotResult> {
        let base_dir = user_base_dir()?;
        let snapshots_dir = base_dir.join("adapter-snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        let created_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("System clock before UNIX_EPOCH")?
            .as_secs();

        let platform = PlatformInfo::current().unwrap_or(PlatformInfo {
            os_id: String::new(),
            os_like: Vec::new(),
            package_manager: None,
        });

        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "gnome".to_string());
        let desktop_key = if desktop.to_lowercase().contains("gnome") {
            "gnome".to_string()
        } else if desktop.to_lowercase().contains("kde") {
            "kde".to_string()
        } else {
            desktop.to_lowercase()
        };

        let enabled_extensions = extensions::list_enabled();
        let enabled_set: HashSet<String> = enabled_extensions.iter().cloned().collect();
        let dconf_dump = dconf::dump("/org/gnome/").unwrap_or_default();
        let dconf_sections_by_id = extract_extension_sections(&dconf_dump, &enabled_extensions);
        let installed_extensions = scan_installed_extensions(&enabled_set)?;

        let tracker_state = LayoutTracker::new()
            .ok()
            .and_then(|tracker| tracker.read_state_json().ok().flatten());

        let snapshot = GnomeAdapterSnapshot {
            meta: SnapshotMeta {
                snapshot_type: "gnome-adapter".to_string(),
                created_at_unix,
                format_version: 1,
            },
            platform: PlatformSnapshot {
                os_id: platform.os_id,
                os_like: platform.os_like,
                package_manager: platform.package_manager,
            },
            session: SessionSnapshot {
                desktop,
                desktop_key,
                session_type: std::env::var("XDG_SESSION_TYPE").ok(),
                gnome_shell_version: detect_gnome_shell_version(),
            },
            tools: ToolsSnapshot {
                gsettings: tools::command_exists("gsettings"),
                dconf: tools::command_exists("dconf"),
                busctl: tools::command_exists("busctl"),
                gext: tools::command_exists("gext"),
            },
            extensions: ExtensionsSnapshot {
                enabled: enabled_extensions,
                installed: installed_extensions,
                dconf_sections_by_id,
            },
            dconf: DconfSnapshot {
                org_gnome_dump: dconf_dump,
            },
            layout_engine: LayoutEngineSnapshot { tracker_state },
        };

        let output_file = snapshots_dir.join(format!("gnome-adapter-snapshot-{created_at_unix}.json"));
        fs::write(&output_file, serde_json::to_string_pretty(&snapshot)?)?;

        Ok(AdapterSnapshotResult { output_file })
    }
}

fn user_base_dir() -> Result<PathBuf> {
    let mut dir = dirs::data_local_dir().context("Could not find local data dir")?;
    dir.push("desktop-experience-switcher");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn detect_gnome_shell_version() -> Option<String> {
    let output = std::process::Command::new("gnome-shell")
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    stdout
        .split_whitespace()
        .find(|token| token.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|s| s.to_string())
}

fn extract_extension_sections(full_dump: &str, enabled_extensions: &[String]) -> BTreeMap<String, String> {
    enabled_extensions
        .iter()
        .filter_map(|extension_id| {
            let slug = extension_id.split('@').next()?;
            let header = format!("[shell/extensions/{slug}]");
            extract_section(full_dump, &header).map(|section| (extension_id.clone(), section))
        })
        .collect()
}

fn extract_section(full_dump: &str, section_header: &str) -> Option<String> {
    let mut found = false;
    let mut lines = Vec::new();

    for line in full_dump.lines() {
        if line.starts_with('[') {
            if line == section_header {
                found = true;
                lines.push(line.to_string());
                continue;
            }
            if found {
                break;
            }
        } else if found {
            lines.push(line.to_string());
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn scan_installed_extensions(enabled_set: &HashSet<String>) -> Result<Vec<InstalledExtensionSnapshot>> {
    let mut results = Vec::new();
    let mut dirs_to_scan = vec![("/usr/share/gnome-shell/extensions", "system")];

    if let Some(home) = dirs::home_dir() {
        let user_path = home.join(".local/share/gnome-shell/extensions");
        if user_path.exists() {
            scan_extension_dir(&user_path, "user", enabled_set, &mut results)?;
        }
    }

    for (path, source) in dirs_to_scan.drain(..) {
        let dir = Path::new(path);
        if dir.exists() {
            scan_extension_dir(dir, source, enabled_set, &mut results)?;
        }
    }

    results.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(results)
}

fn scan_extension_dir(
    dir: &Path,
    source: &str,
    enabled_set: &HashSet<String>,
    results: &mut Vec<InstalledExtensionSnapshot>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = match path.file_name().and_then(|name| name.to_str()) {
            Some(value) => value.to_string(),
            None => continue,
        };
        let metadata_path = path.join("metadata.json");
        let metadata = if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path).ok();
            content.and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        } else {
            None
        };
        let slug = id.split('@').next().unwrap_or(&id).to_string();

        results.push(InstalledExtensionSnapshot {
            id: id.clone(),
            slug,
            enabled: enabled_set.contains(&id),
            source: source.to_string(),
            path: path.to_string_lossy().to_string(),
            metadata,
        });
    }
    Ok(())
}
