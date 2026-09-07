use anyhow::{Context, Result};
use std::process::Command;

use crate::gsettings;
use crate::logging::{log_enter, log_err, log_ok, preview_str};

pub fn execute_bash(command: &str) -> Result<String> {
    let started_at = log_enter("executor::execute_bash", &format!("command={}", preview_str(command)));
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()
        .with_context(|| format!("Failed to execute bash command: {}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    log::info!(
        "executor::execute_bash: exit_code={} stdout={} stderr={}",
        output.status.code().unwrap_or(-1),
        preview_str(&stdout),
        preview_str(&stderr)
    );

    if !output.status.success() {
        let error = anyhow::anyhow!(
            "Bash command failed (exit code: {}):\nStdout: {}\nStderr: {}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        );
        log_err("executor::execute_bash", started_at, &error);
        return Err(error);
    }

    log_ok(
        "executor::execute_bash",
        started_at,
        &format!("stdout={}", preview_str(&stdout)),
    );
    Ok(stdout)
}

pub fn verify_bash(command: &str) -> Result<bool> {
    let started_at = log_enter("executor::verify_bash", &format!("command={}", preview_str(command)));
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()
        .with_context(|| format!("Failed to verify bash command: {}", command))?;

    let ok = output.status.success();
    log_ok(
        "executor::verify_bash",
        started_at,
        &format!(
            "result={} exit_code={} stdout={} stderr={}",
            ok,
            output.status.code().unwrap_or(-1),
            preview_str(&String::from_utf8_lossy(&output.stdout)),
            preview_str(&String::from_utf8_lossy(&output.stderr))
        ),
    );
    Ok(ok)
}

pub fn verify_gsetting(schema: &str, key: &str, expected: &str) -> Result<bool> {
    let started_at = log_enter(
        "executor::verify_gsetting",
        &format!("schema={} key={} expected={}", schema, key, preview_str(expected)),
    );
    let current = gsettings::get(schema, key)?;
    let current_trimmed = current.trim();
    let expected_trimmed = expected.trim();
    let result = current_trimmed == expected_trimmed;
    log_ok(
        "executor::verify_gsetting",
        started_at,
        &format!(
            "result={} current={} expected={}",
            result,
            preview_str(current_trimmed),
            preview_str(expected_trimmed)
        ),
    );
    Ok(result)
}
