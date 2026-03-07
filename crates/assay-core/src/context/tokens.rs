//! Token extraction (exact + heuristic), quick_token_estimate, model context window map.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use assay_types::context::{ContextHealth, SessionEntry, TokenEstimate, UsageData};

use super::parser::ParsedEntry;

/// Default context window size for all current Claude models.
pub(crate) const DEFAULT_CONTEXT_WINDOW: u64 = 200_000;

/// Estimated system overhead (system prompt, tool definitions, etc.).
pub(crate) const SYSTEM_OVERHEAD_TOKENS: u64 = 21_000;

/// Look up context window size for a model string.
///
/// All current Claude models use 200K tokens. Returns the default.
pub fn context_window_for_model(_model: Option<&str>) -> u64 {
    DEFAULT_CONTEXT_WINDOW
}

/// Check whether a session entry belongs to a sidechain (subagent) conversation.
pub(super) fn is_sidechain(entry: &SessionEntry) -> bool {
    match entry {
        SessionEntry::User(e) => e.meta.is_sidechain,
        SessionEntry::Assistant(e) => e.meta.is_sidechain,
        SessionEntry::Progress(e) => e.meta.is_sidechain,
        SessionEntry::System(e) => e.meta.is_sidechain,
        _ => false,
    }
}

/// Extract the latest usage data from the last non-sidechain assistant entry.
pub fn extract_usage(entries: &[ParsedEntry]) -> Option<UsageData> {
    entries
        .iter()
        .rev()
        .filter(|e| !is_sidechain(&e.entry))
        .filter_map(|e| match &e.entry {
            SessionEntry::Assistant(a) => a.message.as_ref()?.usage.clone(),
            _ => None,
        })
        .next()
}

/// Extract the model name from the last assistant entry with a model field.
pub fn extract_model(entries: &[ParsedEntry]) -> Option<String> {
    entries
        .iter()
        .rev()
        .filter(|e| !is_sidechain(&e.entry))
        .filter_map(|e| match &e.entry {
            SessionEntry::Assistant(a) => a.message.as_ref()?.model.clone(),
            _ => None,
        })
        .next()
}

/// Heuristic token estimate from byte count.
///
/// Uses the empirical ratio of ~3.7 bytes per token for English text.
pub(crate) fn estimate_tokens_from_bytes(bytes: u64) -> u64 {
    (bytes as f64 / 3.7).ceil() as u64
}

/// Quick token estimate by reading only the tail of a session file.
///
/// Reads the last 50KB, finds the last assistant entry with usage data.
/// Returns `None` if no usage data is found in the tail.
pub fn quick_token_estimate(path: &Path) -> std::io::Result<Option<UsageData>> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let read_size = file_size.min(50 * 1024) as usize;

    if read_size == 0 {
        return Ok(None);
    }

    // Seek to the tail
    let seek_pos = file_size.saturating_sub(read_size as u64);
    file.seek(SeekFrom::Start(seek_pos))?;

    let mut buf = vec![0u8; read_size];
    file.read_exact(&mut buf)?;

    let text = String::from_utf8_lossy(&buf);

    // Find the last assistant entry with usage data by scanning lines in reverse
    let mut last_usage = None;
    for line in text.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(SessionEntry::Assistant(a)) = serde_json::from_str::<SessionEntry>(line)
            && !a.meta.is_sidechain
            && let Some(msg) = a.message
            && let Some(usage) = msg.usage
        {
            last_usage = Some(usage);
            break;
        }
    }

    Ok(last_usage)
}

/// Full token estimation for a session file.
///
/// Returns a `TokenEstimate` with context utilization percentage and health indicator.
pub fn estimate_tokens(path: &Path, session_id: &str) -> crate::Result<TokenEstimate> {
    let usage = quick_token_estimate(path)
        .map_err(|source| crate::AssayError::Io {
            operation: "reading session file for token estimate".into(),
            path: path.to_path_buf(),
            source,
        })?
        .ok_or_else(|| crate::AssayError::SessionParse {
            path: path.to_path_buf(),
            line: 0,
            message: "no usage data found in session file".into(),
        })?;

    let context_window = DEFAULT_CONTEXT_WINDOW;
    let available = context_window.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
    let context_tokens = usage.context_tokens();
    let pct = (context_tokens as f64 / available as f64) * 100.0;

    let health = if pct < 60.0 {
        ContextHealth::Healthy
    } else if pct < 85.0 {
        ContextHealth::Warning
    } else {
        ContextHealth::Critical
    };

    Ok(TokenEstimate {
        session_id: session_id.to_string(),
        context_tokens,
        output_tokens: usage.output_tokens,
        context_window,
        context_utilization_pct: pct,
        health,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{AssistantEntry, AssistantMessage, EntryMetadata};

    fn make_meta(is_sidechain: bool) -> EntryMetadata {
        EntryMetadata {
            uuid: "test".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: "s1".into(),
            parent_uuid: None,
            is_sidechain,
            cwd: None,
            version: None,
        }
    }

    fn make_assistant_entry(
        is_sidechain: bool,
        usage: Option<UsageData>,
        model: Option<String>,
    ) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::Assistant(AssistantEntry {
                meta: make_meta(is_sidechain),
                message: Some(AssistantMessage {
                    model,
                    content: vec![],
                    usage,
                    stop_reason: None,
                }),
            }),
            line_number: 1,
            raw_bytes: 100,
            raw_line: String::new(),
        }
    }

    #[test]
    fn extract_usage_returns_last_non_sidechain() {
        let entries = vec![
            make_assistant_entry(
                false,
                Some(UsageData {
                    input_tokens: 100,
                    output_tokens: 50,
                    ..Default::default()
                }),
                None,
            ),
            make_assistant_entry(
                true,
                Some(UsageData {
                    input_tokens: 9999,
                    output_tokens: 9999,
                    ..Default::default()
                }),
                None,
            ),
            make_assistant_entry(
                false,
                Some(UsageData {
                    input_tokens: 200,
                    output_tokens: 75,
                    ..Default::default()
                }),
                None,
            ),
        ];

        let usage = extract_usage(&entries).unwrap();
        assert_eq!(usage.input_tokens, 200);
        assert_eq!(usage.output_tokens, 75);
    }

    #[test]
    fn extract_usage_skips_sidechain_only() {
        let entries = vec![make_assistant_entry(
            true,
            Some(UsageData {
                input_tokens: 100,
                ..Default::default()
            }),
            None,
        )];
        assert!(extract_usage(&entries).is_none());
    }

    #[test]
    fn estimate_tokens_from_bytes_calculation() {
        assert_eq!(estimate_tokens_from_bytes(370), 100);
        assert_eq!(estimate_tokens_from_bytes(0), 0);
        // 100 / 3.7 = 27.027... -> ceil -> 28
        assert_eq!(estimate_tokens_from_bytes(100), 28);
    }

    #[test]
    fn context_window_always_200k() {
        assert_eq!(context_window_for_model(None), 200_000);
        assert_eq!(
            context_window_for_model(Some("claude-sonnet-4-5-20250514")),
            200_000
        );
    }
}
