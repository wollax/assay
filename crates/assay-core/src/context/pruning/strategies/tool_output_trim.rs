//! Tool-output-trim strategy: truncates large tool result content.
//!
//! Large tool results (e.g., file reads, command output) inflate sessions.
//! This strategy keeps the first and last N lines with a truncation marker.

use std::collections::HashSet;

use assay_types::context::PrescriptionTier;

use super::super::super::parser::ParsedEntry;
use super::super::strategy::StrategyResult;

/// Lines above which a tool result is considered "large" and eligible for trimming.
const TRIM_THRESHOLD_LINES: usize = 100;

/// Number of lines to keep from the beginning of a large tool result.
const KEEP_HEAD_LINES: usize = 20;

/// Number of lines to keep from the end of a large tool result.
const KEEP_TAIL_LINES: usize = 20;

/// Truncate large tool result content in user entries.
///
/// For each User entry containing `tool_result` blocks whose text content
/// exceeds `TRIM_THRESHOLD_LINES`, replaces the content with the first
/// `KEEP_HEAD_LINES` + a truncation marker + the last `KEEP_TAIL_LINES`.
///
/// Protected entries are preserved unchanged and counted in `protected_skipped`.
pub fn tool_output_trim(
    _entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    _protected: &HashSet<usize>,
) -> StrategyResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{AssistantEntry, EntryMetadata, SessionEntry, UserEntry};

    fn make_meta() -> EntryMetadata {
        EntryMetadata {
            uuid: "test-uuid".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: "s1".into(),
            parent_uuid: None,
            is_sidechain: false,
            cwd: None,
            version: None,
        }
    }

    fn make_user_with_tool_result(line: usize, content: &str) -> ParsedEntry {
        let entry = SessionEntry::User(UserEntry {
            meta: make_meta(),
            message: Some(serde_json::json!([
                {
                    "type": "tool_result",
                    "tool_use_id": "t1",
                    "content": content
                }
            ])),
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes,
            raw_line,
        }
    }

    fn make_user_plain(line: usize) -> ParsedEntry {
        let entry = SessionEntry::User(UserEntry {
            meta: make_meta(),
            message: Some(serde_json::json!("just a text message")),
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes,
            raw_line,
        }
    }

    fn make_assistant(line: usize) -> ParsedEntry {
        let entry = SessionEntry::Assistant(AssistantEntry {
            meta: make_meta(),
            message: None,
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes,
            raw_line,
        }
    }

    fn generate_lines(n: usize) -> String {
        (1..=n)
            .map(|i| format!("line {i} of output"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn small_tool_result_unchanged() {
        let content = generate_lines(10);
        let entries = vec![make_user_with_tool_result(1, &content)];
        let original_raw_line = entries[0].raw_line.clone();
        let result = tool_output_trim(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].raw_line, original_raw_line);
        assert_eq!(result.lines_modified, 0);
        assert_eq!(result.bytes_saved, 0);
    }

    #[test]
    fn large_tool_result_truncated() {
        let content = generate_lines(150);
        let entries = vec![make_user_with_tool_result(1, &content)];
        let original_bytes = entries[0].raw_bytes;
        let result = tool_output_trim(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert!(result.entries[0].raw_bytes < original_bytes);
        assert_eq!(result.lines_modified, 1);
        assert!(result.bytes_saved > 0);

        // Verify truncation marker is present
        assert!(result.entries[0].raw_line.contains("lines truncated"));
    }

    #[test]
    fn user_without_tool_result_unchanged() {
        let entries = vec![make_user_plain(1)];
        let original_raw_line = entries[0].raw_line.clone();
        let result = tool_output_trim(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries[0].raw_line, original_raw_line);
        assert_eq!(result.lines_modified, 0);
    }

    #[test]
    fn non_user_entries_unchanged() {
        let entries = vec![make_assistant(1)];
        let original_raw_line = entries[0].raw_line.clone();
        let result = tool_output_trim(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries[0].raw_line, original_raw_line);
        assert_eq!(result.lines_modified, 0);
    }

    #[test]
    fn protected_entry_not_modified() {
        let mut protected = HashSet::new();
        protected.insert(1);
        let content = generate_lines(150);
        let entries = vec![make_user_with_tool_result(1, &content)];
        let original_raw_line = entries[0].raw_line.clone();
        let result = tool_output_trim(entries, PrescriptionTier::Aggressive, &protected);
        assert_eq!(result.entries[0].raw_line, original_raw_line);
        assert_eq!(result.protected_skipped, 1);
        assert_eq!(result.lines_modified, 0);
        assert_eq!(result.bytes_saved, 0);
    }

    #[test]
    fn multiple_tool_results_each_trimmed() {
        let large_content = generate_lines(150);
        let small_content = generate_lines(10);
        let entry = SessionEntry::User(UserEntry {
            meta: make_meta(),
            message: Some(serde_json::json!([
                {
                    "type": "tool_result",
                    "tool_use_id": "t1",
                    "content": large_content
                },
                {
                    "type": "tool_result",
                    "tool_use_id": "t2",
                    "content": small_content
                },
                {
                    "type": "tool_result",
                    "tool_use_id": "t3",
                    "content": large_content
                }
            ])),
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        let entries = vec![ParsedEntry {
            entry,
            line_number: 1,
            raw_bytes,
            raw_line,
        }];
        let result = tool_output_trim(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.lines_modified, 1);
        assert!(result.bytes_saved > 0);

        // The raw_line should contain two truncation markers (for t1 and t3)
        let marker_count = result.entries[0]
            .raw_line
            .matches("lines truncated")
            .count();
        assert_eq!(marker_count, 2);
    }

    #[test]
    fn bytes_saved_reflects_actual_reduction() {
        let content = generate_lines(200);
        let entries = vec![make_user_with_tool_result(1, &content)];
        let original_bytes = entries[0].raw_bytes;
        let result = tool_output_trim(entries, PrescriptionTier::Aggressive, &HashSet::new());
        let new_bytes = result.entries[0].raw_bytes;
        assert_eq!(result.bytes_saved, (original_bytes - new_bytes) as u64);
    }
}
