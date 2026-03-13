//! Token extraction (exact + heuristic), quick_token_estimate, model context window map.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use assay_types::context::{ContextHealth, GrowthRate, SessionEntry, TokenEstimate, UsageData};

use super::parser::ParsedEntry;
use super::parser::parse_session;

/// Default context window size for all current Claude models.
pub(crate) const DEFAULT_CONTEXT_WINDOW: u64 = 200_000;

/// Estimated system overhead (system prompt, tool definitions, etc.).
pub(crate) const SYSTEM_OVERHEAD_TOKENS: u64 = 21_000;

/// Minimum number of assistant turns required to compute growth rate metrics.
const MIN_TURNS_FOR_GROWTH_RATE: usize = 5;

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

/// Collect context_tokens from each non-sidechain assistant turn with usage data.
///
/// Returns a Vec of cumulative context token counts, one per qualifying turn,
/// in chronological order.
fn collect_turn_tokens_from_entries(entries: &[ParsedEntry]) -> Vec<u64> {
    entries
        .iter()
        .filter(|e| !is_sidechain(&e.entry))
        .filter_map(|e| match &e.entry {
            SessionEntry::Assistant(a) => {
                a.message.as_ref()?.usage.as_ref().map(|u| u.context_tokens())
            }
            _ => None,
        })
        .collect()
}

/// Compute growth rate from turn token snapshots.
///
/// Returns `None` when fewer than `MIN_TURNS_FOR_GROWTH_RATE` turns exist.
/// Uses total context tokens divided by turn count for average growth,
/// then estimates remaining turns from available context budget.
fn compute_growth_rate(turn_tokens: &[u64], context_window: u64) -> Option<GrowthRate> {
    if turn_tokens.len() < MIN_TURNS_FOR_GROWTH_RATE {
        return None;
    }
    let turn_count = turn_tokens.len() as u64;
    let last = *turn_tokens.last()?;
    let avg = last / turn_count;
    let available = context_window.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
    let remaining_tokens = available.saturating_sub(last);
    let remaining_turns = if avg > 0 {
        remaining_tokens / avg
    } else {
        0
    };

    Some(GrowthRate {
        avg_tokens_per_turn: avg,
        estimated_turns_remaining: remaining_turns,
        turn_count,
    })
}

/// Full token estimation for a session file.
///
/// Returns a `TokenEstimate` with context utilization percentage, health indicator,
/// and growth rate metrics (when 5+ assistant turns exist).
pub fn estimate_tokens(path: &Path, session_id: &str) -> crate::Result<TokenEstimate> {
    // Single full parse — extracts both latest usage and per-turn token snapshots.
    let (entries, _) = parse_session(path)?;

    let usage = extract_usage(&entries).ok_or_else(|| crate::AssayError::SessionParse {
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

    // Compute growth rate from the already-parsed entries.
    let turn_tokens = collect_turn_tokens_from_entries(&entries);
    let growth_rate = compute_growth_rate(&turn_tokens, context_window);

    Ok(TokenEstimate {
        session_id: session_id.to_string(),
        context_tokens,
        output_tokens: usage.output_tokens,
        context_window,
        context_utilization_pct: pct,
        health,
        growth_rate,
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

    // ── compute_growth_rate tests ────────────────────────────────────

    #[test]
    fn compute_growth_rate_returns_none_below_threshold() {
        assert!(compute_growth_rate(&[], DEFAULT_CONTEXT_WINDOW).is_none());
        assert!(compute_growth_rate(&[1000], DEFAULT_CONTEXT_WINDOW).is_none());
        assert!(
            compute_growth_rate(&[1000, 2000, 3000, 4000], DEFAULT_CONTEXT_WINDOW).is_none()
        );
    }

    #[test]
    fn compute_growth_rate_returns_some_at_threshold() {
        // 5 turns, last = 5000, avg = 5000/5 = 1000
        // available = 200000 - 21000 = 179000
        // remaining_tokens = 179000 - 5000 = 174000
        // remaining_turns = 174000 / 1000 = 174
        let tokens = vec![1000, 2000, 3000, 4000, 5000];
        let gr = compute_growth_rate(&tokens, DEFAULT_CONTEXT_WINDOW).unwrap();
        assert_eq!(gr.turn_count, 5);
        assert_eq!(gr.avg_tokens_per_turn, 1000);
        assert_eq!(gr.estimated_turns_remaining, 174);
    }

    #[test]
    fn compute_growth_rate_calculates_correctly() {
        // 5 turns, last = 10000, avg = 10000/5 = 2000
        // available = 200000 - 21000 = 179000
        // remaining_tokens = 179000 - 10000 = 169000
        // remaining_turns = 169000 / 2000 = 84
        let tokens = vec![2000, 4000, 6000, 8000, 10000];
        let gr = compute_growth_rate(&tokens, DEFAULT_CONTEXT_WINDOW).unwrap();
        assert_eq!(gr.avg_tokens_per_turn, 2000);
        assert_eq!(gr.estimated_turns_remaining, 84);
        assert_eq!(gr.turn_count, 5);
    }

    #[test]
    fn compute_growth_rate_saturates_when_full() {
        // Context exceeds available window
        // avg = 200000/5 = 40000
        let tokens = vec![50000, 100000, 150000, 180000, 200000];
        let gr = compute_growth_rate(&tokens, DEFAULT_CONTEXT_WINDOW).unwrap();
        assert_eq!(gr.avg_tokens_per_turn, 40000);
        assert_eq!(gr.estimated_turns_remaining, 0);
    }

    #[test]
    fn compute_growth_rate_handles_zero_avg() {
        // All turns had 0 tokens — avg is 0, remaining is 0
        let tokens = vec![0, 0, 0, 0, 0];
        let gr = compute_growth_rate(&tokens, DEFAULT_CONTEXT_WINDOW).unwrap();
        assert_eq!(gr.avg_tokens_per_turn, 0);
        assert_eq!(gr.estimated_turns_remaining, 0);
        assert_eq!(gr.turn_count, 5);
    }

    #[test]
    fn collect_turn_tokens_filters_sidechains() {
        use std::io::Write;

        // Build a session file with mixed entries
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-session.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();

        // Non-sidechain assistant with usage
        let entry1 = serde_json::json!({
            "type": "assistant",
            "uuid": "a1", "timestamp": "2026-01-01T00:00:00Z",
            "sessionId": "s1", "isSidechain": false,
            "message": {
                "content": [], "usage": {
                    "input_tokens": 100, "output_tokens": 10,
                    "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0
                }
            }
        });
        writeln!(file, "{}", entry1).unwrap();

        // Sidechain assistant with usage (should be excluded)
        let entry2 = serde_json::json!({
            "type": "assistant",
            "uuid": "a2", "timestamp": "2026-01-01T00:01:00Z",
            "sessionId": "s1", "isSidechain": true,
            "message": {
                "content": [], "usage": {
                    "input_tokens": 9999, "output_tokens": 99,
                    "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0
                }
            }
        });
        writeln!(file, "{}", entry2).unwrap();

        // Non-sidechain assistant with usage
        let entry3 = serde_json::json!({
            "type": "assistant",
            "uuid": "a3", "timestamp": "2026-01-01T00:02:00Z",
            "sessionId": "s1", "isSidechain": false,
            "message": {
                "content": [], "usage": {
                    "input_tokens": 200, "output_tokens": 20,
                    "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0
                }
            }
        });
        writeln!(file, "{}", entry3).unwrap();

        // User entry (should be excluded)
        let entry4 = serde_json::json!({
            "type": "user",
            "uuid": "u1", "timestamp": "2026-01-01T00:03:00Z",
            "sessionId": "s1", "isSidechain": false
        });
        writeln!(file, "{}", entry4).unwrap();

        let (entries, _) = parse_session(&path).unwrap();
        let tokens = collect_turn_tokens_from_entries(&entries);
        // Should only include the two non-sidechain assistant entries
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], 100); // context_tokens() = input + cache_creation + cache_read
        assert_eq!(tokens[1], 200);
    }
}
