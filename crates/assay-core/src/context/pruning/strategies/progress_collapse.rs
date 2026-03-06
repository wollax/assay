//! Progress-collapse strategy: removes all Progress tick entries.
//!
//! Progress entries (hook_progress, agent_progress, bash_progress) are
//! high-frequency, low-value entries that inflate session files. This
//! strategy removes them entirely, preserving only protected entries.

use std::collections::HashSet;

use assay_types::context::{PrescriptionTier, PruneSample};

use super::super::super::parser::ParsedEntry;
use super::super::strategy::StrategyResult;

/// Remove all unprotected Progress entries from the session.
///
/// Protected progress entries (those whose `line_number` appears in `protected`)
/// are preserved and counted in `protected_skipped`. Up to 3 samples are
/// collected for dry-run display.
pub fn progress_collapse(
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
        if matches!(entry.entry, assay_types::context::SessionEntry::Progress(_)) {
            if protected.contains(&entry.line_number) {
                protected_skipped += 1;
                result_entries.push(entry);
            } else {
                let entry_bytes = entry.raw_bytes as u64;
                if samples.len() < 3 {
                    samples.push(PruneSample {
                        line_number: entry.line_number,
                        description: "Progress tick".into(),
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
        AssistantEntry, EntryMetadata, ProgressEntry, SessionEntry, UserEntry,
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

    fn make_progress(line: usize, bytes: usize) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::Progress(ProgressEntry {
                meta: make_meta(),
                data: Some(serde_json::json!({"type": "bash_progress", "command": "cargo build"})),
            }),
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
    fn empty_entries_returns_empty_result() {
        let result = progress_collapse(vec![], PrescriptionTier::Gentle, &HashSet::new());
        assert!(result.entries.is_empty());
        assert_eq!(result.bytes_saved, 0);
        assert_eq!(result.lines_removed, 0);
        assert_eq!(result.protected_skipped, 0);
        assert!(result.samples.is_empty());
    }

    #[test]
    fn all_progress_entries_removed() {
        let entries = vec![make_progress(1, 100), make_progress(2, 200)];
        let result = progress_collapse(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert!(result.entries.is_empty());
        assert_eq!(result.bytes_saved, 300);
        assert_eq!(result.lines_removed, 2);
    }

    #[test]
    fn mixed_entries_only_progress_removed() {
        let entries = vec![
            make_user(1, 50),
            make_progress(2, 100),
            make_assistant(3, 75),
            make_progress(4, 200),
        ];
        let result = progress_collapse(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 2);
        assert!(matches!(result.entries[0].entry, SessionEntry::User(_)));
        assert!(matches!(
            result.entries[1].entry,
            SessionEntry::Assistant(_)
        ));
        assert_eq!(result.bytes_saved, 300);
        assert_eq!(result.lines_removed, 2);
    }

    #[test]
    fn protected_progress_entry_kept() {
        let mut protected = HashSet::new();
        protected.insert(2);
        let entries = vec![make_progress(1, 100), make_progress(2, 200)];
        let result = progress_collapse(entries, PrescriptionTier::Gentle, &protected);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].line_number, 2);
        assert_eq!(result.protected_skipped, 1);
        assert_eq!(result.bytes_saved, 100);
        assert_eq!(result.lines_removed, 1);
    }

    #[test]
    fn samples_capped_at_three() {
        let entries = vec![
            make_progress(1, 10),
            make_progress(2, 20),
            make_progress(3, 30),
            make_progress(4, 40),
            make_progress(5, 50),
        ];
        let result = progress_collapse(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.samples.len(), 3);
        assert_eq!(result.lines_removed, 5);
    }

    #[test]
    fn samples_contain_correct_data() {
        let entries = vec![make_progress(7, 150), make_progress(12, 250)];
        let result = progress_collapse(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.samples.len(), 2);
        assert_eq!(result.samples[0].line_number, 7);
        assert_eq!(result.samples[0].description, "Progress tick");
        assert_eq!(result.samples[0].bytes, 150);
        assert_eq!(result.samples[1].line_number, 12);
        assert_eq!(result.samples[1].bytes, 250);
    }
}
