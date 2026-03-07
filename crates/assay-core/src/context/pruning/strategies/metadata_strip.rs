//! Metadata-strip strategy: removes structural metadata entries.
//!
//! Removes System, FileHistorySnapshot, QueueOperation, and PrLink entries
//! that inflate session files without contributing to conversation content.

use std::collections::HashSet;

use assay_types::context::{PrescriptionTier, PruneSample, SessionEntry};

use super::super::super::parser::ParsedEntry;
use super::super::strategy::StrategyResult;

/// Returns true if this entry is a metadata type that should be stripped.
fn is_metadata(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::System(_)
            | SessionEntry::FileHistorySnapshot(_)
            | SessionEntry::QueueOperation(_)
            | SessionEntry::PrLink(_)
    )
}

/// Label for a metadata entry type (used in sample descriptions).
fn metadata_label(entry: &SessionEntry) -> &'static str {
    match entry {
        SessionEntry::System(_) => "System entry",
        SessionEntry::FileHistorySnapshot(_) => "File history snapshot",
        SessionEntry::QueueOperation(_) => "Queue operation",
        SessionEntry::PrLink(_) => "PR link",
        _ => "Metadata",
    }
}

/// Remove all unprotected metadata entries from the session.
///
/// Targets: `SessionEntry::System`, `SessionEntry::FileHistorySnapshot`,
/// `SessionEntry::QueueOperation`, and `SessionEntry::PrLink`.
///
/// Protected entries are preserved and counted in `protected_skipped`.
pub fn metadata_strip(
    entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    protected: &HashSet<usize>,
) -> StrategyResult {
    let mut result_entries = Vec::new();
    let mut lines_removed: usize = 0;
    let mut bytes_saved: u64 = 0;
    let mut protected_skipped: usize = 0;
    let mut samples: Vec<PruneSample> = Vec::new();

    for entry in entries {
        if is_metadata(&entry.entry) {
            if protected.contains(&entry.line_number) {
                protected_skipped += 1;
                result_entries.push(entry);
            } else {
                let entry_bytes = entry.raw_bytes as u64;
                if samples.len() < 3 {
                    samples.push(PruneSample {
                        line_number: entry.line_number,
                        description: metadata_label(&entry.entry).into(),
                        bytes: entry_bytes,
                    });
                }
                bytes_saved += entry_bytes;
                lines_removed += 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{
        AssistantEntry, EntryMetadata, SessionEntry, SystemEntry, UserEntry,
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

    fn make_system(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::System(SystemEntry {
                meta: make_meta(),
                data: Some(serde_json::json!({"type": "compact_boundary"})),
            }),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    fn make_file_history_snapshot(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::FileHistorySnapshot(serde_json::json!({
                "type": "file-history-snapshot",
                "uuid": "fhs1",
                "timestamp": "2026-01-01T00:00:00Z",
                "sessionId": "s1"
            })),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    fn make_queue_operation(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::QueueOperation(serde_json::json!({
                "type": "queue-operation",
                "uuid": "qo1",
                "timestamp": "2026-01-01T00:00:00Z",
                "sessionId": "s1"
            })),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    fn make_pr_link(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::PrLink(serde_json::json!({
                "type": "pr-link",
                "uuid": "pr1",
                "timestamp": "2026-01-01T00:00:00Z",
                "sessionId": "s1"
            })),
            line_number: line,
            raw_bytes: bytes,
            raw_line: "x".repeat(bytes),
        }
    }

    fn make_user(line: usize, bytes: usize) -> ParsedEntry {
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

    #[test]
    fn system_entries_removed() {
        let entries = vec![make_system(1, 100), make_system(2, 200)];
        let result = metadata_strip(entries, PrescriptionTier::Standard, &HashSet::new());
        assert!(result.entries.is_empty());
        assert_eq!(result.lines_removed, 2);
        assert_eq!(result.bytes_saved, 300);
    }

    #[test]
    fn file_history_snapshot_removed() {
        let entries = vec![make_file_history_snapshot(1, 500)];
        let result = metadata_strip(entries, PrescriptionTier::Standard, &HashSet::new());
        assert!(result.entries.is_empty());
        assert_eq!(result.lines_removed, 1);
        assert_eq!(result.bytes_saved, 500);
    }

    #[test]
    fn queue_operation_removed() {
        let entries = vec![make_queue_operation(1, 300)];
        let result = metadata_strip(entries, PrescriptionTier::Standard, &HashSet::new());
        assert!(result.entries.is_empty());
        assert_eq!(result.lines_removed, 1);
        assert_eq!(result.bytes_saved, 300);
    }

    #[test]
    fn pr_link_removed() {
        let entries = vec![make_pr_link(1, 250)];
        let result = metadata_strip(entries, PrescriptionTier::Standard, &HashSet::new());
        assert!(result.entries.is_empty());
        assert_eq!(result.lines_removed, 1);
        assert_eq!(result.bytes_saved, 250);
    }

    #[test]
    fn user_and_assistant_preserved() {
        let entries = vec![
            make_user(1, 50),
            make_system(2, 100),
            make_assistant(3, 75),
            make_file_history_snapshot(4, 200),
        ];
        let result = metadata_strip(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 2);
        assert!(matches!(result.entries[0].entry, SessionEntry::User(_)));
        assert!(matches!(
            result.entries[1].entry,
            SessionEntry::Assistant(_)
        ));
        assert_eq!(result.lines_removed, 2);
        assert_eq!(result.bytes_saved, 300);
    }

    #[test]
    fn protected_system_entry_kept() {
        let mut protected = HashSet::new();
        protected.insert(1);
        let entries = vec![make_system(1, 100), make_system(2, 200)];
        let result = metadata_strip(entries, PrescriptionTier::Standard, &protected);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].line_number, 1);
        assert_eq!(result.protected_skipped, 1);
        assert_eq!(result.lines_removed, 1);
        assert_eq!(result.bytes_saved, 200);
    }

    #[test]
    fn unknown_entries_preserved() {
        let entries = vec![ParsedEntry {
            entry: SessionEntry::Unknown,
            line_number: 1,
            raw_bytes: 50,
            raw_line: "x".repeat(50),
        }];
        let result = metadata_strip(entries, PrescriptionTier::Standard, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert!(matches!(result.entries[0].entry, SessionEntry::Unknown));
        assert_eq!(result.lines_removed, 0);
    }
}
