//! Tool-output-trim strategy: truncates large tool result content.
//!
//! Large tool results (e.g., file reads, command output) inflate sessions.
//! This strategy keeps the first and last N lines with a truncation marker.

use std::collections::HashSet;

use assay_types::context::{EntryMetadata, PrescriptionTier, PruneSample, SessionEntry, UserEntry};

use super::super::super::parser::ParsedEntry;
use super::super::strategy::StrategyResult;

/// Lines above which a tool result is considered "large" and eligible for trimming.
const TRIM_THRESHOLD_LINES: usize = 100;

/// Number of lines to keep from the beginning of a large tool result.
const KEEP_HEAD_LINES: usize = 20;

/// Number of lines to keep from the end of a large tool result.
const KEEP_TAIL_LINES: usize = 20;

/// Truncate a string if it exceeds the line threshold.
///
/// Returns `Some(truncated)` if truncation occurred, `None` otherwise.
fn truncate_content(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= TRIM_THRESHOLD_LINES {
        return None;
    }

    let truncated_count = lines.len() - KEEP_HEAD_LINES - KEEP_TAIL_LINES;
    let head = &lines[..KEEP_HEAD_LINES];
    let tail = &lines[lines.len() - KEEP_TAIL_LINES..];

    let mut result = String::new();
    for line in head {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(&format!("[...{truncated_count} lines truncated...]\n"));
    for (i, line) in tail.iter().enumerate() {
        result.push_str(line);
        if i < tail.len() - 1 {
            result.push('\n');
        }
    }

    Some(result)
}

/// Try to trim tool_result blocks in a user entry's message.
///
/// Returns `Some(new_message_value)` if any blocks were trimmed, `None` otherwise.
fn trim_user_message(message: &serde_json::Value) -> Option<serde_json::Value> {
    let blocks = message.as_array()?;
    let mut modified = false;
    let mut new_blocks: Vec<serde_json::Value> = Vec::with_capacity(blocks.len());

    for block in blocks {
        let block_type = block.get("type").and_then(|t| t.as_str());
        if block_type == Some("tool_result")
            && let Some(content_str) = block.get("content").and_then(|c| c.as_str())
            && let Some(truncated) = truncate_content(content_str)
        {
            let mut new_block = block.clone();
            new_block["content"] = serde_json::Value::String(truncated);
            new_blocks.push(new_block);
            modified = true;
            continue;
        }
        new_blocks.push(block.clone());
    }

    modified.then_some(serde_json::Value::Array(new_blocks))
}

/// Extract a trimmed message from a user entry, if applicable.
///
/// Returns `Some((new_message, cloned_meta))` if any tool result blocks were
/// trimmed, `None` if the entry is not a user entry or has no trimmable content.
fn extract_trimmed_message(
    entry: &ParsedEntry,
) -> Option<(serde_json::Value, EntryMetadata)> {
    let SessionEntry::User(u) = &entry.entry else {
        return None;
    };
    let msg = u.message.as_ref()?;
    let new_msg = trim_user_message(msg)?;
    Some((new_msg, u.meta.clone()))
}

/// Truncate large tool result content in user entries.
///
/// For each User entry containing `tool_result` blocks whose text content
/// exceeds `TRIM_THRESHOLD_LINES`, replaces the content with the first
/// `KEEP_HEAD_LINES` + a truncation marker + the last `KEEP_TAIL_LINES`.
///
/// Protected entries are preserved unchanged and counted in `protected_skipped`.
pub fn tool_output_trim(
    entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    protected: &HashSet<usize>,
) -> StrategyResult {
    let mut result_entries = Vec::with_capacity(entries.len());
    let mut lines_modified: usize = 0;
    let mut bytes_saved: u64 = 0;
    let mut protected_skipped: usize = 0;
    let mut samples: Vec<PruneSample> = Vec::new();

    for mut entry in entries {
        // Extract trimmed message if this is a user entry with trimmable tool results.
        let trimmed = extract_trimmed_message(&entry);

        match trimmed {
            None => {
                // Not a user entry with tool results, or content is small enough.
                result_entries.push(entry);
            }
            Some(trimmed_msg) if protected.contains(&entry.line_number) => {
                drop(trimmed_msg);
                protected_skipped += 1;
                result_entries.push(entry);
            }
            Some((new_msg, meta)) => {
                let original_bytes = entry.raw_bytes;
                let new_entry = SessionEntry::User(UserEntry {
                    meta,
                    message: Some(new_msg),
                });
                entry.update_content(new_entry);

                let saved = original_bytes.saturating_sub(entry.raw_bytes) as u64;
                bytes_saved += saved;
                lines_modified += 1;

                if samples.len() < 3 {
                    samples.push(PruneSample {
                        line_number: entry.line_number,
                        description: "Tool output trimmed".into(),
                        bytes: saved,
                    });
                }

                result_entries.push(entry);
            }
        }
    }

    StrategyResult {
        entries: result_entries,
        lines_removed: 0,
        lines_modified,
        bytes_saved,
        protected_skipped,
        samples,
    }
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
