use anyhow::{Context, Result};
use std::process::Command;

use crate::logging::{log_enter, log_err, log_ok, preview_str};

pub fn get(schema: &str, key: &str) -> Result<String> {
    let started_at = log_enter("gsettings::get", &format!("schema={} key={}", schema, key));
    let output = Command::new("gsettings")
        .arg("get")
        .arg(schema)
        .arg(key)
        .output()
        .with_context(|| format!("Failed to execute gsettings get {} {}", schema, key))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let error = anyhow::anyhow!("gsettings get failed: {}", err_msg);
        log_err("gsettings::get", started_at, &error);
        return Err(error);
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    log_ok(
        "gsettings::get",
        started_at,
        &format!("value={}", preview_str(&value)),
    );
    Ok(value)
}

pub fn set(schema: &str, key: &str, value: &str) -> Result<()> {
    let started_at = log_enter(
        "gsettings::set",
        &format!("schema={} key={} value={}", schema, key, preview_str(value)),
    );
    let output = Command::new("gsettings")
        .arg("set")
        .arg(schema)
        .arg(key)
        .arg(value)
        .output()
        .with_context(|| format!("Failed to execute gsettings set {} {} {}", schema, key, value))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let error = anyhow::anyhow!("gsettings set failed: {}", err_msg);
        log_err("gsettings::set", started_at, &error);
        return Err(error);
    }

    log_ok("gsettings::set", started_at, "status=success");
    Ok(())
}
