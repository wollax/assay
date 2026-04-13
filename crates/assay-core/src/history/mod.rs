//! Run history persistence.
//!
//! Saves, loads, and lists [`GateRunRecord`] files as pretty-printed JSON
//! under `.assay/results/<spec-name>/`. Writes are atomic (tempfile-then-rename)
//! so a crash mid-write never leaves a corrupt JSON file on disk.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use tempfile::NamedTempFile;

use assay_types::{GateRunRecord, GateRunSummary};

pub mod analytics;

use tracing::warn;

use crate::error::{AssayError, Result};

/// Validate that a string is safe to use as a single path component.
///
/// Rejects empty strings, path separators (`/`, `\`), and traversal
/// sequences (`..`) to prevent directory escape.
pub(crate) fn validate_path_component(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
    {
        return Err(AssayError::Io {
            operation: format!("validating {label}"),
            path: PathBuf::from(value),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid {label}: {value:?} (must be a safe filename component)"),
            ),
        });
    }
    Ok(())
}

/// Generate a run ID from a timestamp: `YYYYMMDDTHHMMSSZ-<6-char-hex>`.
///
/// The hex suffix is derived from [`RandomState`] (OS-seeded SipHash)
/// with thread ID and nanosecond counter mixed in, providing 24 bits
/// of collision resistance within the same second.
pub(crate) fn generate_run_id(timestamp: &DateTime<Utc>) -> String {
    let ts = timestamp.format("%Y%m%dT%H%M%SZ");
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64,
    );
    // Mix in thread identity via Debug format (stable API).
    use std::hash::Hash;
    std::thread::current().id().hash(&mut hasher);
    let suffix = format!("{:06x}", hasher.finish() & 0xFF_FFFF);
    format!("{ts}-{suffix}")
}

/// Persist a gate run as a [`GateRunRecord`] with auto-generated run ID.
///
/// This is the primary public API for saving gate run history. It handles
/// run ID generation, timestamp capture, and record construction internally,
/// then delegates to [`save()`] for atomic persistence and optional pruning.
pub fn save_run(
    assay_dir: &Path,
    summary: GateRunSummary,
    working_dir: Option<String>,
    max_history: Option<usize>,
) -> Result<SaveResult> {
    let timestamp = chrono::Utc::now();
    let run_id = generate_run_id(&timestamp);
    let record = GateRunRecord {
        run_id,
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        working_dir,
        summary,
        diff_truncation: None,
        precondition_blocked: None,
    };
    save(assay_dir, &record, max_history)
}

/// Result of a save operation, including any pruning that occurred.
#[derive(Debug)]
pub struct SaveResult {
    /// Path to the saved JSON file.
    pub path: PathBuf,
    /// Number of old run files that were pruned.
    pub pruned: usize,
}

/// Remove oldest run files beyond `max_count` for a given spec.
///
/// Calls [`list()`] to get sorted IDs, then deletes the oldest files
/// (those at the beginning of the sorted list) until only `max_count` remain.
/// Returns the number of files deleted.
fn prune(assay_dir: &Path, spec_name: &str, max_count: usize) -> Result<usize> {
    let ids = list(assay_dir, spec_name)?;
    if ids.len() <= max_count {
        return Ok(0);
    }
    let to_remove = ids.len() - max_count;
    let spec_dir = assay_dir.join("results").join(spec_name);
    let mut removed = 0;
    for id in ids.iter().take(to_remove) {
        let path = spec_dir.join(format!("{id}.json"));
        if let Err(e) = std::fs::remove_file(&path) {
            warn!(path = %path.display(), error = %e, "Could not prune history file");
        } else {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Persist a [`GateRunRecord`] as pretty-printed JSON.
///
/// Writes atomically via tempfile-then-rename: a temporary file is created
/// in the target directory, written and fsynced, then renamed to the final
/// path. This guarantees that the JSON file is either fully written or absent.
///
/// Auto-creates `.assay/results/<spec-name>/` on first save.
///
/// When `max_history` is `Some(n)` with `n > 0`, prunes oldest run files
/// for this spec so that at most `n` files remain after the save. A value
/// of `Some(0)` or `None` disables pruning.
///
/// Returns a [`SaveResult`] containing the path and pruning count.
pub fn save(
    assay_dir: &Path,
    record: &GateRunRecord,
    max_history: Option<usize>,
) -> Result<SaveResult> {
    validate_path_component(&record.summary.spec_name, "spec name")?;
    validate_path_component(&record.run_id, "run ID")?;
    let results_dir = assay_dir.join("results").join(&record.summary.spec_name);

    std::fs::create_dir_all(&results_dir).map_err(|source| AssayError::Io {
        operation: "creating results directory".into(),
        path: results_dir.clone(),
        source,
    })?;

    let json = serde_json::to_string_pretty(record)
        .map_err(|e| AssayError::json("serializing gate run record", results_dir.clone(), e))?;

    let mut tmpfile = NamedTempFile::new_in(&results_dir).map_err(|source| AssayError::Io {
        operation: "creating temp file for atomic write".into(),
        path: results_dir.clone(),
        source,
    })?;

    tmpfile
        .write_all(json.as_bytes())
        .map_err(|source| AssayError::Io {
            operation: "writing gate run record".into(),
            path: results_dir.clone(),
            source,
        })?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|source| AssayError::Io {
            operation: "syncing gate run record".into(),
            path: results_dir.clone(),
            source,
        })?;

    let final_path = results_dir.join(format!("{}.json", record.run_id));

    tmpfile.persist(&final_path).map_err(|e| AssayError::Io {
        operation: "persisting gate run record".into(),
        path: final_path.clone(),
        source: e.error,
    })?;

    let pruned = match max_history {
        Some(0) | None => 0,
        Some(max) => prune(assay_dir, &record.summary.spec_name, max)?,
    };

    Ok(SaveResult {
        path: final_path,
        pruned,
    })
}

/// Load a single [`GateRunRecord`] by spec name and run ID.
///
/// The `deny_unknown_fields` on [`GateRunRecord`] enforces strict
/// deserialization — records produced by a different schema version
/// will fail loudly.
pub fn load(assay_dir: &Path, spec_name: &str, run_id: &str) -> Result<GateRunRecord> {
    validate_path_component(spec_name, "spec name")?;
    validate_path_component(run_id, "run ID")?;
    let path = assay_dir
        .join("results")
        .join(spec_name)
        .join(format!("{run_id}.json"));

    let content = std::fs::read_to_string(&path).map_err(|source| AssayError::Io {
        operation: "reading gate run record".into(),
        path: path.clone(),
        source,
    })?;

    serde_json::from_str(&content)
        .map_err(|e| AssayError::json("deserializing gate run record", path, e))
}

/// Return whether the most recent gate run for a spec passed all required criteria.
///
/// Returns `Some(true)` if the latest run had zero required failures,
/// `Some(false)` if the latest run had one or more required failures,
/// and `None` if there is no recorded history for the spec (spec dir missing
/// or empty).
///
/// Callers checking a precondition should map `None` to `false` via
/// `.unwrap_or(false)` — no history means the spec has never passed.
pub fn last_gate_passed(assay_dir: &Path, spec_name: &str) -> Option<bool> {
    let ids = list(assay_dir, spec_name).ok()?;
    let latest_id = ids.last()?;
    let record = load(assay_dir, spec_name, latest_id).ok()?;
    Some(record.summary.enforcement.required_failed == 0)
}

/// List run IDs for a spec, sorted chronologically (oldest first).
///
/// Returns an empty vec if the spec results directory does not exist
/// (this is not an error — the spec simply has no history yet).
pub fn list(assay_dir: &Path, spec_name: &str) -> Result<Vec<String>> {
    validate_path_component(spec_name, "spec name")?;
    let spec_dir = assay_dir.join("results").join(spec_name);

    if !spec_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut ids: Vec<String> = std::fs::read_dir(&spec_dir)
        .map_err(|source| AssayError::Io {
            operation: "listing run history".into(),
            path: spec_dir.clone(),
            source,
        })?
        .filter_map(|entry| match entry {
            Ok(e) => Some(e),
            Err(e) => {
                warn!(error = %e, "Skipping unreadable history entry");
                None
            }
        })
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                path.file_stem().and_then(|s| s.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();

    // Timestamp prefix makes lexicographic sort = chronological sort.
    ids.sort();
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{
        CriterionResult, Enforcement, EnforcementSummary, GateKind, GateResult, GateRunSummary,
    };
    use tempfile::TempDir;

    /// Build a minimal `GateRunSummary` for testing.
    fn make_test_summary(spec_name: &str) -> GateRunSummary {
        GateRunSummary {
            spec_name: spec_name.to_string(),
            results: Vec::new(),
            passed: 1,
            failed: 0,
            skipped: 0,
            total_duration_ms: 42,
            enforcement: EnforcementSummary::default(),
        }
    }

    /// Build a `GateRunRecord` with a known timestamp.
    fn make_test_record(spec_name: &str) -> GateRunRecord {
        let ts = Utc::now();
        GateRunRecord {
            run_id: generate_run_id(&ts),
            assay_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: ts,
            working_dir: None,
            summary: make_test_summary(spec_name),
            diff_truncation: None,
            precondition_blocked: None,
        }
    }

    #[test]
    fn test_generate_run_id_format() {
        let ts = Utc::now();
        let id = generate_run_id(&ts);

        // Pattern: YYYYMMDDTHHMMSSZ-xxxxxx
        let re = regex_lite_match(&id);
        assert!(re, "run_id should match YYYYMMDDTHHMMSSZ-xxxxxx, got: {id}");
    }

    /// Manual regex-lite check (avoids pulling in the regex crate).
    fn regex_lite_match(id: &str) -> bool {
        let parts: Vec<&str> = id.splitn(2, '-').collect();
        if parts.len() != 2 {
            return false;
        }
        let ts_part = parts[0];
        let hex_part = parts[1];

        // Timestamp: 16 chars like 20260305T143600Z
        if ts_part.len() != 16 {
            return false;
        }
        if !ts_part.ends_with('Z') || !ts_part.contains('T') {
            return false;
        }

        // Hex suffix: exactly 6 lowercase hex chars
        hex_part.len() == 6 && hex_part.chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn test_save_creates_file() {
        let dir = TempDir::new().unwrap();
        let record = make_test_record("save-test");

        let result = save(dir.path(), &record, None).unwrap();

        assert!(
            result.path.exists(),
            "saved file should exist at {:?}",
            result.path
        );
        assert_eq!(result.path.extension().unwrap(), "json");
    }

    #[test]
    fn test_save_creates_directories() {
        let dir = TempDir::new().unwrap();
        let record = make_test_record("new-spec");

        // results/new-spec/ does not exist yet
        let spec_dir = dir.path().join("results").join("new-spec");
        assert!(!spec_dir.exists());

        let result = save(dir.path(), &record, None).unwrap();
        assert!(result.path.exists());
        assert!(spec_dir.is_dir());
    }

    #[test]
    fn test_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let record = make_test_record("roundtrip-spec");

        save(dir.path(), &record, None).unwrap();

        let loaded = load(dir.path(), "roundtrip-spec", &record.run_id).unwrap();

        assert_eq!(loaded.run_id, record.run_id);
        assert_eq!(loaded.assay_version, record.assay_version);
        assert_eq!(loaded.timestamp, record.timestamp);
        assert_eq!(loaded.working_dir, record.working_dir);
        assert_eq!(loaded.summary.spec_name, record.summary.spec_name);
        assert_eq!(loaded.summary.passed, record.summary.passed);
        assert_eq!(loaded.summary.failed, record.summary.failed);
        assert_eq!(loaded.summary.skipped, record.summary.skipped);
        assert_eq!(
            loaded.summary.total_duration_ms,
            record.summary.total_duration_ms
        );
    }

    #[test]
    fn test_list_empty_dir() {
        let dir = TempDir::new().unwrap();

        let ids = list(dir.path(), "nonexistent-spec").unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_list_returns_sorted() {
        let dir = TempDir::new().unwrap();

        // Create records with distinct run_ids by generating them separately.
        let r1 = make_test_record("sorted-spec");
        let r2 = make_test_record("sorted-spec");

        save(dir.path(), &r1, None).unwrap();
        save(dir.path(), &r2, None).unwrap();

        let ids = list(dir.path(), "sorted-spec").unwrap();
        assert_eq!(ids.len(), 2);

        // Verify sorted (lexicographic = chronological due to timestamp prefix).
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "list() should return sorted run IDs");
    }

    #[test]
    fn test_save_does_not_clobber() {
        let dir = TempDir::new().unwrap();

        let r1 = make_test_record("clobber-spec");
        let r2 = make_test_record("clobber-spec");

        let r1_result = save(dir.path(), &r1, None).unwrap();
        let r2_result = save(dir.path(), &r2, None).unwrap();

        assert_ne!(
            r1_result.path, r2_result.path,
            "two saves should produce distinct files"
        );
        assert!(r1_result.path.exists());
        assert!(r2_result.path.exists());

        // Both deserialize correctly.
        let l1 = load(dir.path(), "clobber-spec", &r1.run_id).unwrap();
        let l2 = load(dir.path(), "clobber-spec", &r2.run_id).unwrap();
        assert_eq!(l1.run_id, r1.run_id);
        assert_eq!(l2.run_id, r2.run_id);
    }

    #[test]
    fn test_concurrent_saves_produce_distinct_files() {
        use std::sync::Arc;
        use std::thread;

        let dir = TempDir::new().unwrap();
        let dir_path = Arc::new(dir.path().to_path_buf());
        let spec_name = "concurrent-spec";

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let path = Arc::clone(&dir_path);
                let name = spec_name.to_string();
                thread::spawn(move || {
                    let record = make_test_record(&name);
                    let result = save(&path, &record, None).unwrap();
                    (record.run_id, result.path)
                })
            })
            .collect();

        let results: Vec<(String, PathBuf)> =
            handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All 10 saves produced files.
        assert_eq!(results.len(), 10);

        // All paths are distinct (no clobbering).
        let paths: std::collections::HashSet<_> = results.iter().map(|(_, p)| p.clone()).collect();
        assert_eq!(paths.len(), 10, "all 10 file paths should be distinct");

        // list() returns exactly 10 entries.
        let ids = list(&dir_path, spec_name).unwrap();
        assert_eq!(ids.len(), 10, "list() should return 10 run IDs");

        // Each entry deserializes successfully.
        for id in &ids {
            let loaded = load(&dir_path, spec_name, id).unwrap();
            assert_eq!(&loaded.run_id, id);
        }
    }

    #[test]
    fn test_partial_write_leaves_no_corrupt_file() {
        let dir = TempDir::new().unwrap();
        let spec_name = "crash-spec";
        let results_dir = dir.path().join("results").join(spec_name);
        std::fs::create_dir_all(&results_dir).unwrap();

        // Simulate a crash: write truncated JSON to a temp file in the results dir.
        // Use a name that looks like a temp file (no .json extension).
        let crash_file = results_dir.join(".tmp_partial_write");
        std::fs::write(&crash_file, r#"{"run_id":"broken","assay_ver"#).unwrap();

        // list() should NOT include the temp file (no .json extension).
        let ids = list(dir.path(), spec_name).unwrap();
        assert!(ids.is_empty(), "temp file should not appear in list()");

        // A valid save still works fine alongside the debris.
        let record = make_test_record(spec_name);
        let result = save(dir.path(), &record, None).unwrap();
        assert!(result.path.exists());

        // Load the valid record.
        let loaded = load(dir.path(), spec_name, &record.run_id).unwrap();
        assert_eq!(loaded.run_id, record.run_id);
    }

    #[test]
    fn test_full_fidelity_roundtrip() {
        let dir = TempDir::new().unwrap();

        let timestamp = DateTime::parse_from_rfc3339("2026-03-05T14:30:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let record = GateRunRecord {
            run_id: generate_run_id(&timestamp),
            assay_version: "0.2.0-test".to_string(),
            timestamp,
            working_dir: Some("/tmp/test-project".into()),
            diff_truncation: None,
            precondition_blocked: None,
            summary: GateRunSummary {
                spec_name: "fidelity-spec".to_string(),
                results: vec![
                    CriterionResult {
                        criterion_name: "cargo-test".to_string(),
                        result: Some(GateResult {
                            passed: true,
                            kind: GateKind::Command {
                                cmd: "cargo test".to_string(),
                            },
                            stdout: "test result: ok. 42 passed".to_string(),
                            stderr: String::new(),
                            exit_code: Some(0),
                            duration_ms: 1500,
                            timestamp,
                            truncated: false,
                            original_bytes: None,
                            evidence: None,
                            reasoning: None,
                            confidence: None,
                            evaluator_role: None,
                        }),
                        enforcement: Enforcement::Required,
                        source: None,
                    },
                    CriterionResult {
                        criterion_name: "lint-check".to_string(),
                        result: Some(GateResult {
                            passed: false,
                            kind: GateKind::Command {
                                cmd: "cargo clippy".to_string(),
                            },
                            stdout: String::new(),
                            stderr: "warning: unused variable".to_string(),
                            exit_code: Some(1),
                            duration_ms: 800,
                            timestamp,
                            truncated: true,
                            original_bytes: Some(131_072),
                            evidence: None,
                            reasoning: None,
                            confidence: None,
                            evaluator_role: None,
                        }),
                        enforcement: Enforcement::Advisory,
                        source: None,
                    },
                    CriterionResult {
                        criterion_name: "descriptive-only".to_string(),
                        result: None,
                        enforcement: Enforcement::Required,
                        source: None,
                    },
                    CriterionResult {
                        criterion_name: "readme-exists".to_string(),
                        result: Some(GateResult {
                            passed: true,
                            kind: GateKind::FileExists {
                                path: "README.md".to_string(),
                            },
                            stdout: String::new(),
                            stderr: String::new(),
                            exit_code: None,
                            duration_ms: 1,
                            timestamp,
                            truncated: false,
                            original_bytes: None,
                            evidence: None,
                            reasoning: None,
                            confidence: None,
                            evaluator_role: None,
                        }),
                        enforcement: Enforcement::Required,
                        source: None,
                    },
                ],
                passed: 2,
                failed: 1,
                skipped: 1,
                total_duration_ms: 2301,
                enforcement: EnforcementSummary {
                    required_passed: 2,
                    required_failed: 0,
                    advisory_passed: 0,
                    advisory_failed: 1,
                },
            },
        };

        // Save and load back.
        save(dir.path(), &record, None).unwrap();
        let loaded = load(dir.path(), "fidelity-spec", &record.run_id).unwrap();

        // Full structural equality.
        assert_eq!(record, loaded, "roundtrip should preserve all fields");

        // Independent deserialization from raw JSON.
        let raw_path = dir
            .path()
            .join("results")
            .join("fidelity-spec")
            .join(format!("{}.json", record.run_id));
        let raw_json = std::fs::read_to_string(&raw_path).unwrap();
        let independent: GateRunRecord = serde_json::from_str(&raw_json).unwrap();
        assert_eq!(
            record, independent,
            "independent deserialization should match original"
        );
    }

    #[test]
    fn test_path_traversal_rejected() {
        let dir = TempDir::new().unwrap();

        // spec_name with traversal
        let mut record = make_test_record("../escape");
        assert!(save(dir.path(), &record, None).is_err());

        // spec_name with slash
        record.summary.spec_name = "foo/bar".to_string();
        assert!(save(dir.path(), &record, None).is_err());

        // load with traversal
        assert!(load(dir.path(), "..", "some-id").is_err());
        assert!(load(dir.path(), "ok-spec", "../escape").is_err());

        // list with traversal
        assert!(list(dir.path(), "..").is_err());
    }

    /// Build a `GateRunRecord` with a specific timestamp for deterministic ordering.
    fn make_test_record_at(spec_name: &str, timestamp: DateTime<Utc>) -> GateRunRecord {
        GateRunRecord {
            run_id: generate_run_id(&timestamp),
            assay_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp,
            working_dir: None,
            summary: make_test_summary(spec_name),
            diff_truncation: None,
            precondition_blocked: None,
        }
    }

    #[test]
    fn test_prune_removes_oldest() {
        use chrono::Duration;

        let dir = TempDir::new().unwrap();
        let spec = "prune-oldest";
        let base = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // Save 5 records with distinct timestamps (1-second offsets).
        let mut run_ids = Vec::new();
        for i in 0..5 {
            let ts = base + Duration::seconds(i);
            let record = make_test_record_at(spec, ts);
            run_ids.push(record.run_id.clone());
            save(dir.path(), &record, None).unwrap();
        }

        // Prune to keep only 3.
        let pruned = prune(dir.path(), spec, 3).unwrap();
        assert_eq!(pruned, 2, "should prune 2 oldest files");

        let remaining = list(dir.path(), spec).unwrap();
        assert_eq!(remaining.len(), 3, "should have 3 files remaining");

        // The 3 newest should remain (ids at index 2, 3, 4).
        for id in &run_ids[2..] {
            assert!(remaining.contains(id), "newest run {id} should remain");
        }
        // The 2 oldest should be gone.
        for id in &run_ids[..2] {
            assert!(!remaining.contains(id), "oldest run {id} should be pruned");
        }
    }

    #[test]
    fn test_prune_zero_means_unlimited() {
        use chrono::Duration;

        let dir = TempDir::new().unwrap();
        let spec = "prune-zero";
        let base = DateTime::parse_from_rfc3339("2026-02-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        for i in 0..5 {
            let ts = base + Duration::seconds(i);
            let record = make_test_record_at(spec, ts);
            save(dir.path(), &record, Some(0)).unwrap();
        }

        let ids = list(dir.path(), spec).unwrap();
        assert_eq!(ids.len(), 5, "max_history=0 should not prune");
    }

    #[test]
    fn test_prune_none_means_no_pruning() {
        use chrono::Duration;

        let dir = TempDir::new().unwrap();
        let spec = "prune-none";
        let base = DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        for i in 0..5 {
            let ts = base + Duration::seconds(i);
            let record = make_test_record_at(spec, ts);
            save(dir.path(), &record, None).unwrap();
        }

        let ids = list(dir.path(), spec).unwrap();
        assert_eq!(ids.len(), 5, "max_history=None should not prune");
    }

    #[test]
    fn test_prune_to_one_keeps_only_latest() {
        use chrono::Duration;

        let dir = TempDir::new().unwrap();
        let spec = "prune-one";
        let base = DateTime::parse_from_rfc3339("2026-05-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let mut run_ids = Vec::new();
        for i in 0..3 {
            let ts = base + Duration::seconds(i);
            let record = make_test_record_at(spec, ts);
            run_ids.push(record.run_id.clone());
            save(dir.path(), &record, None).unwrap();
        }

        let pruned = prune(dir.path(), spec, 1).unwrap();
        assert_eq!(pruned, 2, "should prune 2 of 3 files");

        let remaining = list(dir.path(), spec).unwrap();
        assert_eq!(remaining.len(), 1, "should have exactly 1 file remaining");
        assert_eq!(remaining[0], run_ids[2], "the latest record should remain");
    }

    #[test]
    fn test_save_result_contains_pruned_count() {
        use chrono::Duration;

        let dir = TempDir::new().unwrap();
        let spec = "prune-count";
        let base = DateTime::parse_from_rfc3339("2026-04-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // Save 5 records without pruning.
        for i in 0..5 {
            let ts = base + Duration::seconds(i);
            let record = make_test_record_at(spec, ts);
            save(dir.path(), &record, None).unwrap();
        }

        // Save a 6th with max_history=3 — should prune 3 (6 total, keep 3).
        let ts6 = base + Duration::seconds(5);
        let record6 = make_test_record_at(spec, ts6);
        let result = save(dir.path(), &record6, Some(3)).unwrap();

        assert_eq!(result.pruned, 3, "SaveResult.pruned should be 3");
        assert!(result.path.exists(), "saved file should exist");

        let remaining = list(dir.path(), spec).unwrap();
        assert_eq!(remaining.len(), 3, "should have 3 files remaining");
    }

    #[test]
    fn test_load_file_not_found() {
        let dir = TempDir::new().unwrap();
        let result = load(dir.path(), "nonexistent-spec", "20260101T000000Z-abc123");
        assert!(
            result.is_err(),
            "loading nonexistent file should return error"
        );
    }

    #[test]
    fn test_load_invalid_json() {
        let dir = TempDir::new().unwrap();
        let spec_dir = dir.path().join("results").join("bad-json-spec");
        std::fs::create_dir_all(&spec_dir).unwrap();
        std::fs::write(
            spec_dir.join("20260101T000000Z-abc123.json"),
            "this is not valid JSON {{{",
        )
        .unwrap();

        let result = load(dir.path(), "bad-json-spec", "20260101T000000Z-abc123");
        assert!(result.is_err(), "loading invalid JSON should return error");
    }

    #[test]
    fn test_load_roundtrip_with_results() {
        use assay_types::{CriterionResult, Enforcement, GateKind, GateResult};

        let dir = TempDir::new().unwrap();
        let ts = Utc::now();
        let record = GateRunRecord {
            run_id: generate_run_id(&ts),
            assay_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: ts,
            working_dir: Some("/tmp/test".to_string()),
            diff_truncation: None,
            precondition_blocked: None,
            summary: GateRunSummary {
                spec_name: "results-test".to_string(),
                results: vec![CriterionResult {
                    criterion_name: "unit-tests".to_string(),
                    result: Some(GateResult {
                        passed: true,
                        kind: GateKind::Command {
                            cmd: "cargo test".to_string(),
                        },
                        stdout: "ok".to_string(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration_ms: 100,
                        timestamp: ts,
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                    source: None,
                }],
                passed: 1,
                failed: 0,
                skipped: 0,
                total_duration_ms: 100,
                enforcement: EnforcementSummary {
                    required_passed: 1,
                    required_failed: 0,
                    advisory_passed: 0,
                    advisory_failed: 0,
                },
            },
        };

        let save_result = save(dir.path(), &record, None).unwrap();
        let loaded = load(dir.path(), "results-test", &record.run_id).unwrap();

        assert_eq!(loaded.summary.results.len(), 1);
        assert_eq!(
            loaded.summary.results[0].criterion_name, "unit-tests",
            "criterion name should roundtrip"
        );
        assert!(
            loaded.summary.results[0].result.as_ref().unwrap().passed,
            "result.passed should roundtrip"
        );
        assert_eq!(loaded, record);
        let _ = save_result;
    }

    #[test]
    fn test_save_run_creates_record() {
        let dir = TempDir::new().unwrap();
        let summary = make_test_summary("save-run-spec");

        let result = save_run(dir.path(), summary, Some("/tmp/wd".to_string()), None).unwrap();

        assert!(result.path.exists(), "save_run should create a file");
        assert_eq!(result.pruned, 0);

        // Verify the record can be loaded and has correct fields.
        let ids = list(dir.path(), "save-run-spec").unwrap();
        assert_eq!(ids.len(), 1);

        let loaded = load(dir.path(), "save-run-spec", &ids[0]).unwrap();
        assert_eq!(loaded.summary.spec_name, "save-run-spec");
        assert_eq!(loaded.working_dir, Some("/tmp/wd".to_string()));
        assert_eq!(loaded.assay_version, env!("CARGO_PKG_VERSION"));
        assert!(!loaded.run_id.is_empty());
    }

    #[test]
    fn test_save_run_respects_max_history() {
        let dir = TempDir::new().unwrap();

        for _ in 0..5 {
            let summary = make_test_summary("save-run-prune");
            save_run(dir.path(), summary, None, None).unwrap();
        }

        // Save a 6th with max_history=3.
        let summary = make_test_summary("save-run-prune");
        let result = save_run(dir.path(), summary, None, Some(3)).unwrap();

        assert_eq!(result.pruned, 3, "should prune 3 oldest files");
        let remaining = list(dir.path(), "save-run-prune").unwrap();
        assert_eq!(remaining.len(), 3);
    }

    // ── last_gate_passed tests ────────────────────────────────────────

    /// Build a GateRunSummary with a specific required_failed count.
    fn make_summary_with_failures(spec_name: &str, required_failed: usize) -> GateRunSummary {
        GateRunSummary {
            spec_name: spec_name.to_string(),
            results: Vec::new(),
            passed: if required_failed == 0 { 1 } else { 0 },
            failed: required_failed,
            skipped: 0,
            total_duration_ms: 10,
            enforcement: EnforcementSummary {
                required_passed: if required_failed == 0 { 1 } else { 0 },
                required_failed,
                advisory_passed: 0,
                advisory_failed: 0,
            },
        }
    }

    // Test 1: last_gate_passed returns Some(true) when latest run has required_failed == 0
    #[test]
    fn test_last_gate_passed_returns_some_true_when_passed() {
        let dir = TempDir::new().unwrap();
        let summary = make_summary_with_failures("spec-pass", 0);
        save_run(dir.path(), summary, None, None).unwrap();

        let result = last_gate_passed(dir.path(), "spec-pass");
        assert_eq!(
            result,
            Some(true),
            "should return Some(true) when required_failed == 0"
        );
    }

    // Test 2: last_gate_passed returns Some(false) when latest run has required_failed > 0
    #[test]
    fn test_last_gate_passed_returns_some_false_when_failed() {
        let dir = TempDir::new().unwrap();
        let summary = make_summary_with_failures("spec-fail", 1);
        save_run(dir.path(), summary, None, None).unwrap();

        let result = last_gate_passed(dir.path(), "spec-fail");
        assert_eq!(
            result,
            Some(false),
            "should return Some(false) when required_failed > 0"
        );
    }

    // Test 3: last_gate_passed returns None when spec has no history (empty results dir)
    #[test]
    fn test_last_gate_passed_returns_none_when_no_history() {
        let dir = TempDir::new().unwrap();
        // Create the results/spec-name dir but add no records
        std::fs::create_dir_all(dir.path().join("results").join("empty-spec")).unwrap();

        let result = last_gate_passed(dir.path(), "empty-spec");
        assert_eq!(
            result, None,
            "should return None when spec has no saved runs"
        );
    }

    // Test 4: last_gate_passed returns None when spec directory does not exist
    #[test]
    fn test_last_gate_passed_returns_none_when_spec_dir_absent() {
        let dir = TempDir::new().unwrap();
        // No results dir created at all

        let result = last_gate_passed(dir.path(), "nonexistent-spec");
        assert_eq!(
            result, None,
            "should return None when spec dir does not exist"
        );
    }
}
