//! Bloat categorization, stale read detection, and DiagnosticsReport assembly.

use std::path::Path;

use assay_types::context::{DiagnosticsReport, SessionEntry};

use super::parser::{parse_session, ParsedEntry};
use super::tokens::{context_window_for_model, extract_model, extract_usage, is_sidechain, SYSTEM_OVERHEAD_TOKENS};

/// Analyze a parsed session and produce a `DiagnosticsReport`.
pub fn diagnose(path: &Path, session_id: &str) -> crate::Result<DiagnosticsReport> {
    let (entries, _skipped) = parse_session(path)?;
    let file_size = std::fs::metadata(path)
        .map_err(|source| crate::AssayError::Io {
            operation: "reading session file metadata".into(),
            path: path.to_path_buf(),
            source,
        })?
        .len();

    let usage = extract_usage(&entries);
    let model = extract_model(&entries);
    let context_window = context_window_for_model(model.as_deref());
    let available = context_window.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
    let context_pct = usage
        .as_ref()
        .map(|u| (u.context_tokens() as f64 / available as f64) * 100.0);

    let bloat = categorize_bloat(&entries, file_size);

    let message_count = entries
        .iter()
        .filter(|e| matches!(&e.entry, SessionEntry::User(_) | SessionEntry::Assistant(_)))
        .filter(|e| !is_sidechain(&e.entry))
        .count() as u64;

    Ok(DiagnosticsReport {
        session_id: session_id.to_string(),
        file_path: path.to_string_lossy().to_string(),
        file_size_bytes: file_size,
        total_entries: entries.len() as u64,
        message_count,
        model,
        context_window,
        system_overhead: SYSTEM_OVERHEAD_TOKENS,
        usage,
        context_utilization_pct: context_pct,
        bloat,
    })
}

// Stub — will be fully implemented below
fn categorize_bloat(
    _entries: &[ParsedEntry],
    _file_size: u64,
) -> assay_types::context::BloatBreakdown {
    assay_types::context::BloatBreakdown { entries: vec![] }
}
