use anyhow::{Context, Result};
use infra::{dconf, extensions};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ExportService;

pub struct ExportResult {
    pub export_dir: PathBuf,
    pub user_layout_file: PathBuf,
    pub starter_de_file: PathBuf,
}

#[derive(Serialize)]
struct GnomeExportManifest {
    enabled_extensions: Vec<String>,
    extension_dconf_sections: BTreeMap<String, String>,
    full_dconf_dump_file: String,
    starter_de_file: String,
    user_layout_file: String,
}

impl ExportService {
    pub fn export_current_gnome_layout() -> Result<ExportResult> {
        let base_dir = user_base_dir()?;
        let exports_dir = base_dir.join("exports");
        let user_layouts_dir = base_dir.join("layouts");
        fs::create_dir_all(&exports_dir)?;
        fs::create_dir_all(&user_layouts_dir)?;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("System clock before UNIX_EPOCH")?
            .as_secs();
        let export_dir = exports_dir.join(format!("gnome-layout-export-{ts}"));
        fs::create_dir_all(&export_dir)?;

        let enabled_extensions = extensions::list_enabled();
        let full_dconf_dump = dconf::dump("/org/gnome/")?;
        let extension_sections = extract_extension_sections(&full_dconf_dump, &enabled_extensions);

        let layout_id = format!("custom_gnome_{ts}");
        let starter_de = build_starter_de(&layout_id, &enabled_extensions, &full_dconf_dump);
        let starter_de_file = export_dir.join(format!("{layout_id}.de"));
        let user_layout_file = user_layouts_dir.join(format!("{layout_id}.de"));

        fs::write(export_dir.join("gnome-full.dconf"), &full_dconf_dump)?;
        fs::write(&starter_de_file, &starter_de)?;
        fs::write(&user_layout_file, &starter_de)?;
        fs::write(
            export_dir.join("README.txt"),
            build_readme(&layout_id, &enabled_extensions, &user_layout_file),
        )?;

        let manifest = GnomeExportManifest {
            enabled_extensions,
            extension_dconf_sections: extension_sections,
            full_dconf_dump_file: "gnome-full.dconf".to_string(),
            starter_de_file: starter_de_file
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string(),
            user_layout_file: user_layout_file.to_string_lossy().to_string(),
        };
        fs::write(
            export_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?,
        )?;

        Ok(ExportResult {
            export_dir,
            user_layout_file,
            starter_de_file,
        })
    }
}

fn user_base_dir() -> Result<PathBuf> {
    let mut dir = dirs::data_local_dir().context("Could not find local data dir")?;
    dir.push("desktop-experience-switcher");
    fs::create_dir_all(&dir)?;
    Ok(dir)
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

fn build_starter_de(layout_id: &str, enabled_extensions: &[String], full_dconf_dump: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("id: {layout_id}\n"));
    out.push_str(&format!("label: Custom GNOME {layout_id}\n"));
    out.push_str("preview: gnome\n");
    out.push_str("description: Własny layout wyeksportowany z bieżącej sesji GNOME\n");
    out.push_str("desktops:\n");
    out.push_str("  gnome:\n");
    out.push_str("    min_version: '40'\n");
    out.push_str("    post_flight_wait_ms: 1500\n");
    out.push_str("    steps:\n");

    for extension_id in enabled_extensions {
        out.push_str("      - type: extension.install\n");
        out.push_str(&format!("        id: {extension_id}\n"));
    }

    out.push_str("      - type: dconf\n");
    out.push_str("        path: /org/gnome/\n");
    out.push_str("        action: load\n");
    out.push_str("        data: |\n");
    for line in full_dconf_dump.lines() {
        out.push_str("          ");
        out.push_str(line);
        out.push('\n');
    }

    out.push_str("      - type: gnome.shell_reload\n");
    out.push_str("        wait_ms: 3000\n");
    out
}

fn build_readme(layout_id: &str, enabled_extensions: &[String], user_layout_file: &PathBuf) -> String {
    format!(
        "GNOME export created for manual layout authoring.\n\nLayout ID: {layout_id}\nEnabled extensions: {}\n\nFiles:\n- gnome-full.dconf: full /org/gnome/ dump\n- manifest.json: enabled extensions and extracted per-extension sections\n- {layout_id}.de: starter layout file\n\nEditable user layout path:\n{}\n\nThe app also loads layouts from ~/.local/share/desktop-experience-switcher/layouts, so after editing that .de file, restart the app and your custom layout should appear in the list.\n",
        enabled_extensions.join(", "),
        user_layout_file.to_string_lossy()
    )
}
