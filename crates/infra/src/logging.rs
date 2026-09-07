use std::fmt::Debug;
use std::time::Instant;

pub fn preview_debug<T: Debug>(value: &T) -> String {
    preview_string(&format!("{value:?}"))
}

pub fn preview_str(value: &str) -> String {
    preview_string(value)
}

pub fn preview_lines(value: &str, max_lines: usize) -> String {
    let mut lines: Vec<&str> = value.lines().take(max_lines).collect();
    let total_lines = value.lines().count();
    if total_lines > max_lines {
        lines.push("...");
    }
    preview_string(&lines.join("\\n"))
}

pub fn log_enter(function: &str, context: &str) -> Instant {
    log::info!("[ENTER] {} {}", function, context);
    Instant::now()
}

pub fn log_ok(function: &str, started_at: Instant, context: &str) {
    log::info!(
        "[OK] {} {} elapsed_ms={}",
        function,
        context,
        started_at.elapsed().as_millis()
    );
}

pub fn log_err(function: &str, started_at: Instant, error: &dyn std::fmt::Display) {
    log::error!(
        "[ERR] {} error={} elapsed_ms={}",
        function,
        preview_str(&error.to_string()),
        started_at.elapsed().as_millis()
    );
}

fn preview_string(value: &str) -> String {
    const MAX_CHARS: usize = 240;
    let sanitized = value.replace('\n', "\\n");
    let mut chars = sanitized.chars();
    let collected: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{collected}...(truncated)")
    } else {
        collected
    }
}
