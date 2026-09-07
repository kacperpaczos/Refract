use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use crate::tools::{ensure_tool, TOOL_GEXT};
use crate::logging::{log_enter, log_err, log_ok, preview_debug, preview_str};

/// Instaluje rozszerzenie z internetu (gnome.org) przez `gext install <id>`.
/// Jeśli `gext` jest niedostępne, włączy instalator asystenta GUI (pkexec).
pub fn install_extension(id: &str) -> Result<()> {
    install_extension_with_progress(id, |_phase, _detail| {})
}

pub fn install_extension_with_progress<F>(id: &str, mut on_status: F) -> Result<()>
where
    F: FnMut(&str, &str),
{
    let started_at = log_enter("extensions::install_extension", &format!("id={}", id));
    ensure_tool(&TOOL_GEXT)?;
    on_status(
        "extension.download",
        &format!("Downloading {} from extensions.gnome.org", id),
    );

    let mut child = Command::new("gext")
        .arg("install")
        .arg(id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let started_wait = Instant::now();
    let mut last_heartbeat = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            break;
        }

        if last_heartbeat.elapsed() >= Duration::from_secs(1) {
            on_status(
                "extension.download",
                &format!(
                    "Downloading and unpacking {}... {}s elapsed",
                    id,
                    started_wait.elapsed().as_secs()
                ),
            );
            last_heartbeat = Instant::now();
        }

        thread::sleep(Duration::from_millis(250));
    }

    let output = child.wait_with_output()?;

    log::info!(
        "extensions::install_extension: exit_code={} stdout={} stderr={}",
        output.status.code().unwrap_or(-1),
        preview_str(&String::from_utf8_lossy(&output.stdout)),
        preview_str(&String::from_utf8_lossy(&output.stderr))
    );

    if output.status.success() {
        on_status(
            "extension.install",
            &format!("Downloaded archive for {} and installed it locally", id),
        );
        log_ok("extensions::install_extension", started_at, &format!("id={}", id));
        Ok(())
    } else {
        let error = anyhow::anyhow!("gext failed to install {}", id);
        log_err("extensions::install_extension", started_at, &error);
        Err(error)
    }
}

pub fn install_bundled_extension_with_progress<F>(id: &str, source_dir: &Path, mut on_status: F) -> Result<()>
where
    F: FnMut(&str, &str),
{
    let started_at = log_enter(
        "extensions::install_bundled_extension",
        &format!("id={} source_dir={:?}", id, source_dir),
    );
    let metadata_path = source_dir.join("metadata.json");
    if !metadata_path.exists() {
        let error = anyhow::anyhow!("Bundled extension metadata not found at {:?}", metadata_path);
        log_err("extensions::install_bundled_extension", started_at, &error);
        return Err(error);
    }

    let install_root = gnome_extensions_user_dir()?;
    let dest_dir = install_root.join(id);

    on_status(
        "extension.bundle-copy",
        &format!("Copying bundled files for {}", id),
    );
    fs::create_dir_all(&install_root)
        .with_context(|| format!("Failed to create GNOME extensions dir {:?}", install_root))?;
    if dest_dir.exists() {
        fs::remove_dir_all(&dest_dir)
            .with_context(|| format!("Failed to replace existing extension dir {:?}", dest_dir))?;
    }
    copy_dir_recursive(source_dir, &dest_dir)?;

    on_status(
        "extension.bundle-enable",
        &format!("Enabling bundled extension {}", id),
    );
    set_extension_enabled(id, true)?;

    log_ok("extensions::install_bundled_extension", started_at, &format!("id={}", id));
    Ok(())
}

/// Odinstalowuje rozszerzenie przez `gext uninstall`.
pub fn uninstall_extension(id: &str) -> Result<()> {
    let started_at = log_enter("extensions::uninstall_extension", &format!("id={}", id));
    if let Err(_) = ensure_tool(&TOOL_GEXT) {
        log::warn!("'gext' not found, falling back to just disabling {}", id);
        let result = disable_extension(id);
        if let Err(error) = &result {
            log_err("extensions::uninstall_extension", started_at, error);
        } else {
            log_ok("extensions::uninstall_extension", started_at, "fallback=disable_extension");
        }
        return result;
    }

    let output = Command::new("gext").arg("uninstall").arg(id).output()?;
    log::info!(
        "extensions::uninstall_extension: exit_code={} stdout={} stderr={}",
        output.status.code().unwrap_or(-1),
        preview_str(&String::from_utf8_lossy(&output.stdout)),
        preview_str(&String::from_utf8_lossy(&output.stderr))
    );
    log_ok("extensions::uninstall_extension", started_at, &format!("id={}", id));
    Ok(())
}

/// Włącza zainstalowane rozszerzenie dopisując go na siłę do dconf za pomocą `gext`.
pub fn enable_extension(id: &str) -> Result<()> {
    let started_at = log_enter("extensions::enable_extension", &format!("id={}", id));
    let ensure_result = ensure_tool(&TOOL_GEXT);
    if let Err(error) = &ensure_result {
        log::warn!("extensions::enable_extension: ensure_tool failed for {}: {}", id, error);
    }
    let output = Command::new("gext").arg("enable").arg(id).output()?;
    log_ok(
        "extensions::enable_extension",
        started_at,
        &format!(
            "exit_code={} stdout={} stderr={}",
            output.status.code().unwrap_or(-1),
            preview_str(&String::from_utf8_lossy(&output.stdout)),
            preview_str(&String::from_utf8_lossy(&output.stderr))
        ),
    );
    Ok(())
}

/// Wyłącza rozszerzenie w dconf za pomocą `gext`. Omija błędy 'rozszerzenie nie istnieje'.
pub fn disable_extension(id: &str) -> Result<()> {
    let started_at = log_enter("extensions::disable_extension", &format!("id={}", id));
    let ensure_result = ensure_tool(&TOOL_GEXT);
    if let Err(error) = &ensure_result {
        log::warn!("extensions::disable_extension: ensure_tool failed for {}: {}", id, error);
    }
    let output = Command::new("gext").arg("disable").arg(id).output()?;
    log_ok(
        "extensions::disable_extension",
        started_at,
        &format!(
            "exit_code={} stdout={} stderr={}",
            output.status.code().unwrap_or(-1),
            preview_str(&String::from_utf8_lossy(&output.stdout)),
            preview_str(&String::from_utf8_lossy(&output.stderr))
        ),
    );
    Ok(())
}

/// Zwraca listę aktualnie włączonych rozszerzeń zadeklarowanych w systemie
pub fn list_enabled() -> Vec<String> {
    let started_at = log_enter("extensions::list_enabled", "");
    if let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.shell", "enabled-extensions"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let parsed: Vec<String> = stdout
            .replace("[", "")
            .replace("]", "")
            .replace("'", "")
            .replace("\"", "")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        log_ok(
            "extensions::list_enabled",
            started_at,
            &format!("extensions={}", preview_debug(&parsed)),
        );
        parsed
    } else {
        log::warn!("extensions::list_enabled: gsettings command failed");
        log_ok("extensions::list_enabled", started_at, "extensions=[]");
        Vec::new()
    }
}

fn gnome_extensions_user_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to resolve home directory")?;
    Ok(home.join(".local/share/gnome-shell/extensions"))
}

fn set_extension_enabled(id: &str, enabled: bool) -> Result<()> {
    let mut extensions = list_enabled();
    let already_enabled = extensions.iter().any(|current| current == id);

    if enabled && !already_enabled {
        extensions.push(id.to_string());
    } else if !enabled && already_enabled {
        extensions.retain(|current| current != id);
    } else {
        return Ok(());
    }

    let formatted = format!(
        "[{}]",
        extensions
            .iter()
            .map(|item| format!("'{}'", item))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let output = Command::new("gsettings")
        .args(["set", "org.gnome.shell", "enabled-extensions", &formatted])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to update enabled extensions via gsettings: {}",
            preview_str(&String::from_utf8_lossy(&output.stderr))
        ))
    }
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)
        .with_context(|| format!("Failed to create destination dir {:?}", dest))?;

    for entry in fs::read_dir(source).with_context(|| format!("Failed to read {:?}", source))? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &dest_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &dest_path).with_context(|| {
                format!("Failed to copy file from {:?} to {:?}", source_path, dest_path)
            })?;
            let permissions = fs::metadata(&source_path)?.permissions();
            fs::set_permissions(&dest_path, permissions)?;
        }
    }

    Ok(())
}
