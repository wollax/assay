//! Stale-reads strategy: removes all but the last read of each file path.
//!
//! When a file is read multiple times in a session, only the most recent read
//! is valuable (it reflects the latest file state). Earlier reads are stale
//! and can be safely removed to save context window space.

use std::collections::{HashMap, HashSet};

use assay_types::context::{PrescriptionTier, PruneSample, SessionEntry};

use super::super::super::parser::ParsedEntry;
use super::super::strategy::StrategyResult;

/// Remove all but the last Read of each file path.
///
/// Uses a two-pass approach:
/// 1. Scan all entries to find the last occurrence of each file path read.
/// 2. Remove earlier reads unless they are in the `protected` set.
///
/// Non-read entries pass through unchanged.
pub fn stale_reads(
    entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    protected: &HashSet<usize>,
) -> StrategyResult {
    // First pass: find the last read line_number for each file path.
    let mut last_read: HashMap<String, usize> = HashMap::new();
    for entry in &entries {
        if let Some(path) = extract_read_path(entry) {
            last_read.insert(path, entry.line_number);
        }
    }

    // Second pass: remove stale reads.
    let mut result_entries = Vec::new();
    let mut lines_removed: usize = 0;
    let mut bytes_saved: u64 = 0;
    let mut protected_skipped: usize = 0;
    let mut samples: Vec<PruneSample> = Vec::new();

    for entry in entries {
        if let Some(path) = extract_read_path(&entry) {
            let is_last = last_read.get(&path) == Some(&entry.line_number);
            if !is_last {
                // This is a stale read
                if protected.contains(&entry.line_number) {
                    protected_skipped += 1;
                    result_entries.push(entry);
                } else {
                    let entry_bytes = entry.raw_bytes as u64;
                    if samples.len() < 3 {
                        samples.push(PruneSample {
                            line_number: entry.line_number,
                            description: format!("Stale read: {path}"),
                            bytes: entry_bytes,
                        });
                    }
                    bytes_saved += entry_bytes;
                    lines_removed += 1;
                }
            } else {
                result_entries.push(entry);
            }
        } else {
            result_entries.push(entry);
        }
    }

    StrategyResult {
        entries: result_entries,
        lines_removed,
        lines_modified: 0,
        bytes_saved,
        protected_skipped,
        samples,
    }
}

/// Extract the file path from a Read tool_use block within a User entry.
///
/// Matches the pattern from `diagnostics.rs`: user message as array,
/// block with type "tool_use" and name "Read" or "read", file_path from input.
fn extract_read_path(entry: &ParsedEntry) -> Option<String> {
    let SessionEntry::User(u) = &entry.entry else {
        return None;
    };
    let msg = u.message.as_ref()?;
    let blocks = msg.as_array()?;
    for block in blocks {
        let block_type = block.get("type").and_then(|t| t.as_str());
        if block_type != Some("tool_use") {
            continue;
        }
        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if name != "Read" && name != "read" {
            continue;
        }
        if let Some(path) = block
            .get("input")
            .and_then(|i| i.get("file_path"))
            .and_then(|p| p.as_str())
        {
            return Some(path.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{
        AssistantEntry, EntryMetadata, ProgressEntry, UserEntry,
    };

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

    fn make_read(line: usize, bytes: usize, path: &str, tool_name: &str) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::User(UserEntry {
                meta: make_meta(),
                message: Some(serde_json::json!([
                    {
                        "type": "tool_use",
                        "id": format!("t{line}"),
                        "name": tool_name,
                        "input": { "file_path": path }
                    }
                ])),
            }),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    fn make_user_text(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::User(UserEntry {
                meta: make_meta(),
                message: Some(serde_json::json!("hello")),
            }),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    fn make_assistant(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::Assistant(AssistantEntry {
                meta: make_meta(),
                message: None,
            }),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    fn make_progress(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::Progress(ProgressEntry {
                meta: make_meta(),
                data: None,
            }),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    #[test]
    fn empty_entries_returns_empty_result() {
        let result = stale_reads(vec![], PrescriptionTier::Standard, &HashSet::new());
        assert!(result.entries.is_empty());
        assert_eq!(result.bytes_saved, 0);
        assert_eq!(result.lines_removed, 0);
    }

    #[test]
    fn single_read_kept() {
        let entries = vec![make_read(1, 100, "/src/main.rs", "Read")];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.lines_removed, 0);
        assert_eq!(result.bytes_saved, 0);
    }

    #[test]
    fn two_reads_same_file_first_removed() {
        let entries = vec![
            make_read(1, 100, "/src/main.rs", "Read"),
            make_read(2, 150, "/src/main.rs", "Read"),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].line_number, 2);
        assert_eq!(result.lines_removed, 1);
        assert_eq!(result.bytes_saved, 100);
    }

    #[test]
    fn three_reads_same_file_only_last_kept() {
        let entries = vec![
            make_read(1, 100, "/src/main.rs", "Read"),
            make_read(3, 150, "/src/main.rs", "Read"),
            make_read(5, 200, "/src/main.rs", "Read"),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].line_number, 5);
        assert_eq!(result.lines_removed, 2);
        assert_eq!(result.bytes_saved, 250);
    }

    #[test]
    fn different_files_all_kept() {
        let entries = vec![
            make_read(1, 100, "/src/main.rs", "Read"),
            make_read(2, 100, "/src/lib.rs", "Read"),
            make_read(3, 100, "/src/foo.rs", "Read"),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 3);
        assert_eq!(result.lines_removed, 0);
    }

    #[test]
    fn protected_stale_read_kept() {
        let mut protected = HashSet::new();
        protected.insert(1);
        let entries = vec![
            make_read(1, 100, "/src/main.rs", "Read"),
            make_read(2, 150, "/src/main.rs", "Read"),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &protected);
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.protected_skipped, 1);
        assert_eq!(result.lines_removed, 0);
        assert_eq!(result.bytes_saved, 0);
    }

    #[test]
    fn non_read_user_entries_not_affected() {
        let entries = vec![
            make_user_text(1, 50),
            make_user_text(2, 60),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.lines_removed, 0);
    }

    #[test]
    fn mixed_entries_only_stale_reads_removed() {
        let entries = vec![
            make_user_text(1, 50),
            make_read(2, 100, "/src/main.rs", "Read"),
            make_assistant(3, 75),
            make_read(4, 150, "/src/main.rs", "Read"),
            make_progress(5, 30),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 4); // user_text, assistant, last read, progress
        assert_eq!(result.entries[0].line_number, 1);
        assert_eq!(result.entries[1].line_number, 3);
        assert_eq!(result.entries[2].line_number, 4);
        assert_eq!(result.entries[3].line_number, 5);
        assert_eq!(result.lines_removed, 1);
        assert_eq!(result.bytes_saved, 100);
    }

    #[test]
    fn lowercase_read_tool_detected() {
        let entries = vec![
            make_read(1, 100, "/src/main.rs", "read"),
            make_read(2, 150, "/src/main.rs", "read"),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].line_number, 2);
        assert_eq!(result.lines_removed, 1);
    }

    #[test]
    fn samples_collected_for_removed_entries() {
        let entries = vec![
            make_read(1, 100, "/src/main.rs", "Read"),
            make_read(2, 150, "/src/main.rs", "Read"),
        ];
        let result = stale_reads(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.samples.len(), 1);
        assert_eq!(result.samples[0].line_number, 1);
        assert_eq!(result.samples[0].bytes, 100);
        assert!(result.samples[0].description.contains("/src/main.rs"));
    }
}
