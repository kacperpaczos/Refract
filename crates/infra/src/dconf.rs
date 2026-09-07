use anyhow::{Context, Result};
use std::process::Command;

use crate::logging::{log_enter, log_err, log_ok, preview_lines, preview_str};

pub fn dump(path: &str) -> Result<String> {
    let started_at = log_enter("dconf::dump", &format!("path={}", path));
    let output = Command::new("dconf")
        .arg("dump")
        .arg(path)
        .output()
        .context("Failed to execute dconf dump")?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let error = anyhow::anyhow!("dconf dump failed: {}", err_msg);
        log_err("dconf::dump", started_at, &error);
        return Err(error);
    }

    let dump = String::from_utf8_lossy(&output.stdout).to_string();
    log_ok(
        "dconf::dump",
        started_at,
        &format!("bytes={} preview={}", dump.len(), preview_lines(&dump, 3)),
    );
    Ok(dump)
}

pub fn load(path: &str, data: &str) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    let started_at = log_enter(
        "dconf::load",
        &format!("path={} bytes={} preview={}", path, data.len(), preview_lines(data, 3)),
    );
    let mut child = Command::new("dconf")
        .arg("load")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn dconf load")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(data.as_bytes())
            .context("Failed to write to dconf load stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait on dconf load")?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        // "non-writable keys" — część kluczy systemowych jest chroniona, to normalne podczas restore
        if err_msg.contains("non-writable") {
            log::warn!(
                "dconf load: niektóre klucze są chronione systemowo i zostały pominięte (non-writable). \
                 Pozostałe ustawienia zostały przywrócone."
            );
            log_ok(
                "dconf::load",
                started_at,
                &format!("status=non-writable-warning stderr={}", preview_str(&err_msg)),
            );
            return Ok(());
        }
        let error = anyhow::anyhow!("dconf load failed: {}", err_msg);
        log_err("dconf::load", started_at, &error);
        return Err(error);
    }

    log_ok("dconf::load", started_at, "status=success");
    Ok(())
}
