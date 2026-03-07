//! Composable pruning engine for session JSONL files.
//!
//! Provides a pipeline of strategies that reduce session bloat while
//! preserving team coordination messages. Each strategy is a pure function
//! operating on `Vec<ParsedEntry>`.

pub mod backup;
pub mod protection;
pub mod report;
pub mod strategies;
pub mod strategy;

pub use strategy::{StrategyResult, apply_strategy};

use std::collections::HashSet;
use std::path::Path;

use assay_types::context::{PrescriptionTier, PruneReport, PruneStrategy, PruneSummary};

use super::parser::ParsedEntry;

/// Result of running the full pipeline.
pub struct PipelineResult {
    /// Final entries after all strategies applied.
    pub entries: Vec<ParsedEntry>,
    /// Per-strategy results paired with the strategy that produced them.
    pub strategy_results: Vec<(PruneStrategy, StrategyResult)>,
    /// Total bytes of original entries.
    pub original_size: u64,
    /// Total bytes of final entries.
    pub final_size: u64,
}

/// Execute a pruning pipeline: apply strategies sequentially, collecting results.
///
/// Each strategy operates on the output of the previous one. Per-strategy
/// results are tracked for reporting. The protection set is passed to every
/// strategy so team coordination messages are never modified.
pub fn execute_pipeline(
    _entries: Vec<ParsedEntry>,
    _strategies: &[PruneStrategy],
    _tier: PrescriptionTier,
    _protected_lines: &HashSet<usize>,
) -> PipelineResult {
    todo!("Implemented in GREEN phase")
}

/// Write entries as JSONL to `target` using atomic temp+rename.
///
/// Creates a temporary file in the same directory as `target`, writes all
/// entries' `raw_line` values as lines, then persists atomically. This
/// prevents data loss if the process crashes mid-write.
pub fn write_session(_entries: &[ParsedEntry], _target: &Path) -> crate::Result<()> {
    todo!("Implemented in GREEN phase")
}

/// Top-level pruning API: parse, protect, pipeline, optionally write.
///
/// If `execute` is false (dry-run), the session file is not modified.
/// If `execute` is true, a backup is created first, then the pruned
/// session is written atomically.
pub fn prune_session(
    _session_path: &Path,
    _strategies: &[PruneStrategy],
    _tier: PrescriptionTier,
    _execute: bool,
    _backup_dir: Option<&Path>,
) -> crate::Result<PruneReport> {
    todo!("Implemented in GREEN phase")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{
        EntryMetadata, PrescriptionTier, ProgressEntry, PruneStrategy, SessionEntry, UserEntry,
    };
    use std::io::Write;

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

    fn make_user_entry(line: usize, raw: &str) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::User(UserEntry {
                meta: make_meta(),
                message: Some(serde_json::json!("hello")),
            }),
            line_number: line,
            raw_bytes: raw.len(),
            raw_line: raw.into(),
        }
    }

    fn make_progress_entry(line: usize, raw: &str) -> ParsedEntry {
        ParsedEntry {
            entry: SessionEntry::Progress(ProgressEntry {
                meta: make_meta(),
                data: Some(serde_json::json!({"type": "bash_progress", "command": "cargo build"})),
            }),
            line_number: line,
            raw_bytes: raw.len(),
            raw_line: raw.into(),
        }
    }

    // ---------- execute_pipeline tests ----------

    #[test]
    fn pipeline_empty_strategies_returns_entries_unchanged() {
        let entries = vec![make_user_entry(1, "line1"), make_user_entry(2, "line2")];
        let protected = HashSet::new();
        let result =
            execute_pipeline(entries, &[], PrescriptionTier::Gentle, &protected);

        assert_eq!(result.entries.len(), 2);
        assert!(result.strategy_results.is_empty());
        assert_eq!(result.original_size, result.final_size);
    }

    #[test]
    fn pipeline_single_strategy_transforms_entries() {
        let entries = vec![
            make_user_entry(1, "user-line"),
            make_progress_entry(2, "progress-line"),
            make_user_entry(3, "user-line2"),
        ];
        let protected = HashSet::new();
        let result = execute_pipeline(
            entries,
            &[PruneStrategy::ProgressCollapse],
            PrescriptionTier::Gentle,
            &protected,
        );

        // Progress entry removed
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.strategy_results.len(), 1);
        assert_eq!(result.strategy_results[0].0, PruneStrategy::ProgressCollapse);
        assert_eq!(result.strategy_results[0].1.lines_removed, 1);
    }

    #[test]
    fn pipeline_two_strategies_compose_sequentially() {
        let entries = vec![
            make_user_entry(1, "user-line"),
            make_progress_entry(2, "progress-line"),
            make_progress_entry(3, "progress-line2"),
        ];
        let protected = HashSet::new();
        let result = execute_pipeline(
            entries,
            &[PruneStrategy::ProgressCollapse, PruneStrategy::MetadataStrip],
            PrescriptionTier::Standard,
            &protected,
        );

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.strategy_results.len(), 2);
        assert_eq!(result.strategy_results[0].1.lines_removed, 2);
        assert_eq!(result.strategy_results[1].1.lines_removed, 0);
    }

    #[test]
    fn pipeline_original_and_final_size_calculated() {
        let entries = vec![
            make_user_entry(1, "twelve-byte"),
            make_progress_entry(2, "prog-bytes"),
        ];
        let original_size: u64 = entries.iter().map(|e| e.raw_bytes as u64).sum();
        let protected = HashSet::new();

        let result = execute_pipeline(
            entries,
            &[PruneStrategy::ProgressCollapse],
            PrescriptionTier::Gentle,
            &protected,
        );

        assert_eq!(result.original_size, original_size);
        assert!(result.final_size < result.original_size);
    }

    #[test]
    fn pipeline_protection_set_passed_to_each_strategy() {
        let entries = vec![
            make_progress_entry(1, "protected-progress"),
            make_progress_entry(2, "unprotected-progress"),
        ];
        let mut protected = HashSet::new();
        protected.insert(1);

        let result = execute_pipeline(
            entries,
            &[PruneStrategy::ProgressCollapse],
            PrescriptionTier::Gentle,
            &protected,
        );

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].line_number, 1);
        assert_eq!(result.strategy_results[0].1.protected_skipped, 1);
    }

    #[test]
    fn pipeline_strategy_order_matches_input() {
        let entries = vec![make_user_entry(1, "test")];
        let protected = HashSet::new();

        let strategies = [PruneStrategy::MetadataStrip, PruneStrategy::ProgressCollapse];
        let result = execute_pipeline(entries, &strategies, PrescriptionTier::Standard, &protected);

        assert_eq!(result.strategy_results[0].0, PruneStrategy::MetadataStrip);
        assert_eq!(result.strategy_results[1].0, PruneStrategy::ProgressCollapse);
    }

    // ---------- write_session tests ----------

    #[test]
    fn write_session_writes_jsonl_lines() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("output.jsonl");

        let entries = vec![
            make_user_entry(1, r#"{"type":"user","line":1}"#),
            make_user_entry(2, r#"{"type":"user","line":2}"#),
        ];

        write_session(&entries, &target).unwrap();

        let content = std::fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], r#"{"type":"user","line":1}"#);
        assert_eq!(lines[1], r#"{"type":"user","line":2}"#);
    }

    #[test]
    fn write_session_atomic_file_appears_complete() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("session.jsonl");

        let entries = vec![
            make_user_entry(1, "line-one"),
            make_user_entry(2, "line-two"),
            make_user_entry(3, "line-three"),
        ];

        write_session(&entries, &target).unwrap();

        let content = std::fs::read_to_string(&target).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn write_session_empty_entries_creates_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("empty.jsonl");

        write_session(&[], &target).unwrap();

        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.is_empty());
    }

    // ---------- prune_session tests ----------

    fn write_test_session(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("test-session.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","uuid":"u1","timestamp":"2026-01-01T00:00:00Z","sessionId":"s1"}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"progress","uuid":"p1","timestamp":"2026-01-01T00:00:01Z","sessionId":"s1"}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","uuid":"u2","timestamp":"2026-01-01T00:00:02Z","sessionId":"s1"}}"#
        )
        .unwrap();
        path
    }

    #[test]
    fn prune_session_dry_run_does_not_modify_file() {
        let dir = tempfile::tempdir().unwrap();
        let session_path = write_test_session(dir.path());
        let original = std::fs::read_to_string(&session_path).unwrap();

        let report = prune_session(
            &session_path,
            &[PruneStrategy::ProgressCollapse],
            PrescriptionTier::Gentle,
            false,
            None,
        )
        .unwrap();

        assert!(!report.executed);
        assert!(report.strategies[0].lines_removed > 0);
        let after = std::fs::read_to_string(&session_path).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn prune_session_execute_modifies_file() {
        let dir = tempfile::tempdir().unwrap();
        let session_path = write_test_session(dir.path());
        let original = std::fs::read_to_string(&session_path).unwrap();
        let backup_dir = dir.path().join("backups");

        let report = prune_session(
            &session_path,
            &[PruneStrategy::ProgressCollapse],
            PrescriptionTier::Gentle,
            true,
            Some(&backup_dir),
        )
        .unwrap();

        assert!(report.executed);
        let after = std::fs::read_to_string(&session_path).unwrap();
        assert_ne!(original, after);
        assert!(report.final_entries < report.original_entries);
    }

    #[test]
    fn prune_session_end_to_end_flow() {
        let dir = tempfile::tempdir().unwrap();
        let session_path = write_test_session(dir.path());
        let backup_dir = dir.path().join("backups");

        let report = prune_session(
            &session_path,
            PrescriptionTier::Gentle.strategies(),
            PrescriptionTier::Gentle,
            true,
            Some(&backup_dir),
        )
        .unwrap();

        assert!(report.executed);
        assert_eq!(report.session_id, "test-session");
        assert!(report.original_size > report.final_size);
        assert!(report.original_entries > report.final_entries);
        let backups: Vec<_> = std::fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(!backups.is_empty());
    }
}
