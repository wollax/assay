pub mod checkpoint;
pub mod context;
pub mod gate;
pub mod harness;
pub mod history;
pub mod init;
pub mod manifest;
pub mod mcp;
pub mod milestone;
pub mod plan;
pub mod pr;
pub mod run;
pub mod spec;
pub mod traces;
pub mod wizard_helpers;
pub mod worktree;

use anyhow::Context;
use std::io::IsTerminal;
use std::path::PathBuf;

// ── Shared constants ──────────────────────────────────────────────

/// Extra bytes added by a single ANSI color sequence pair (`\x1b[XXm` ... `\x1b[0m`).
/// `\x1b[32m` = 5 bytes, `\x1b[0m` = 4 bytes, total = 9.
pub(crate) const ANSI_COLOR_OVERHEAD: usize = 9;

/// Column separator used in CLI table output (two spaces).
pub(crate) const COLUMN_GAP: &str = "  "; // 2 spaces

/// Name of the Assay project directory relative to project root.
pub(crate) const ASSAY_DIR_NAME: &str = ".assay";

// ── Shared helpers ────────────────────────────────────────────────

/// Build an absolute path to the Assay project directory under `root`.
pub(crate) fn assay_dir(root: &std::path::Path) -> PathBuf {
    root.join(ASSAY_DIR_NAME)
}

/// Check whether terminal colors should be used.
///
/// Returns `false` when the `NO_COLOR` environment variable is set
/// (any value, including empty — per <https://no-color.org/>) or when
/// stderr is not a terminal (e.g., piped to a file or another process).
/// We check stderr because gate streaming output goes to stderr.
pub(crate) fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}

/// Resolve the project root directory.
pub(crate) fn project_root() -> anyhow::Result<PathBuf> {
    std::env::current_dir().context("could not determine current directory")
}

/// Map a [`GateKind`](assay_types::GateKind) to a short display label for CLI output.
pub(crate) fn gate_kind_label(kind: &assay_types::GateKind) -> &'static str {
    match kind {
        assay_types::GateKind::Command { .. } => "[cmd]",
        assay_types::GateKind::FileExists { .. } => "[file]",
        assay_types::GateKind::AlwaysPass => "[auto]",
        assay_types::GateKind::AgentReport => "[agent]",
        assay_types::GateKind::EventCount { .. } => "[events]",
        assay_types::GateKind::NoToolErrors => "[no-tool-errors]",
    }
}

/// Derive a display label from a [`Criterion`](assay_types::Criterion) struct.
///
/// Uses the same labels as [`gate_kind_label`] but infers kind from criterion fields.
pub(crate) fn criterion_label(criterion: &assay_types::Criterion) -> &'static str {
    if criterion.kind == Some(assay_types::CriterionKind::AgentReport) {
        "[agent]"
    } else if criterion.cmd.is_some() {
        "[cmd]"
    } else if criterion.path.is_some() {
        "[file]"
    } else {
        ""
    }
}

/// Format a criterion type label, optionally with ANSI color.
///
/// "executable" (has a `cmd` or `path`) renders green; "descriptive" renders yellow.
pub(crate) fn format_criteria_type(is_executable: bool, color: bool) -> &'static str {
    if is_executable {
        if color {
            "\x1b[32mexecutable\x1b[0m"
        } else {
            "executable"
        }
    } else if color {
        "\x1b[33mdescriptive\x1b[0m"
    } else {
        "descriptive"
    }
}

/// Format "ok" with optional green color.
pub(crate) fn format_pass(color: bool) -> &'static str {
    if color { "\x1b[32mok\x1b[0m" } else { "ok" }
}

/// Format "FAILED" with optional red color.
pub(crate) fn format_fail(color: bool) -> &'static str {
    if color {
        "\x1b[31mFAILED\x1b[0m"
    } else {
        "FAILED"
    }
}

/// Format "WARN" with optional yellow color.
#[allow(dead_code)]
pub(crate) fn format_warn(color: bool) -> &'static str {
    if color { "\x1b[33mWARN\x1b[0m" } else { "WARN" }
}

/// Format a number with optional ANSI color, only applying color when
/// the value is non-zero.
pub(crate) fn format_count(value: usize, ansi_code: &str, color: bool) -> String {
    if color && value > 0 {
        format!("{ansi_code}{value}\x1b[0m")
    } else {
        value.to_string()
    }
}

/// Format a byte count as a human-readable size string (e.g., "2.4 MB").
pub(crate) fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a number with thousands separators (e.g., 156234 -> "156,234").
pub(crate) fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

/// Shared threshold logic for relative time formatting.
///
/// Returns a relative string like "5s ago", "3m ago", "2h ago", "4d ago",
/// or an absolute `%Y-%m-%d %H:%M` string when `secs` is negative or >= 7 days.
/// The `suffix` is appended to each relative unit (e.g., `" ago"` or `""`).
fn relative_from_secs(secs: i64, dt: &chrono::DateTime<chrono::Utc>, suffix: &str) -> String {
    if secs < 0 {
        return dt.format("%Y-%m-%d %H:%M").to_string();
    }
    if secs < 60 {
        format!("{secs}s{suffix}")
    } else if secs < 3600 {
        format!("{}m{suffix}", secs / 60)
    } else if secs < 86400 {
        format!("{}h{suffix}", secs / 3600)
    } else if secs < 604800 {
        format!("{}d{suffix}", secs / 86400)
    } else {
        dt.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// Format a relative time string from an ISO 8601 timestamp (e.g., "2h ago").
pub(crate) fn format_relative_time(iso: &str) -> String {
    match iso.parse::<chrono::DateTime<chrono::Utc>>() {
        Ok(dt) => {
            let secs = chrono::Utc::now().signed_duration_since(dt).num_seconds();
            relative_from_secs(secs, &dt, " ago")
        }
        Err(_) => iso.to_string(),
    }
}

/// Format a timestamp as a relative age string (e.g., "5m", "2h") or absolute when >= 1 day.
///
/// Unlike [`format_relative_time`] which shows day-relative strings up to 7 days,
/// this function falls back to absolute dates at the 24-hour boundary for compact display.
pub(crate) fn format_relative_timestamp(ts: &chrono::DateTime<chrono::Utc>) -> String {
    let secs = chrono::Utc::now().signed_duration_since(*ts).num_seconds();
    if secs < 0 {
        return ts.format("%Y-%m-%d %H:%M").to_string();
    }
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        ts.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// Format a duration in milliseconds as a human-readable string.
pub(crate) fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        let secs = ms as f64 / 1000.0;
        if ms.is_multiple_of(1000) {
            format!("{secs:.0}s")
        } else {
            format!("{secs:.1}s")
        }
    } else {
        let total_secs = ms / 1000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if secs == 0 {
            format!("{mins}m")
        } else {
            format!("{mins}m {secs}s")
        }
    }
}

/// Color a string with an ANSI code, respecting the `color` flag.
pub(crate) fn colorize(text: &str, ansi_code: &str, color: bool) -> String {
    if color {
        format!("{ansi_code}{text}\x1b[0m")
    } else {
        text.to_string()
    }
}
