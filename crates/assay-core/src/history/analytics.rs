//! Gate history analytics: failure frequency and milestone velocity.
//!
//! Aggregates [`GateRunRecord`] history and [`Milestone`] data to produce
//! an [`AnalyticsReport`] containing per-criterion failure frequency and
//! per-milestone velocity metrics.
//!
//! The compute functions scan `.assay/results/` for history records and
//! `.assay/milestones/` for milestone TOML files. Unreadable records are
//! counted (not fatal) so consumers know if data was skipped.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use assay_types::Enforcement;

use tracing::warn;

use crate::error::Result;

/// How often a specific criterion fails across all recorded gate runs.
///
/// Keyed by `(spec_name, criterion_name)` pair — criteria with the same
/// name in different specs are tracked independently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureFrequency {
    /// Name of the spec this criterion belongs to.
    pub spec_name: String,
    /// Name of the criterion within the spec.
    pub criterion_name: String,
    /// Number of runs where this criterion failed.
    pub fail_count: usize,
    /// Total number of runs where this criterion was evaluated (passed or failed).
    pub total_runs: usize,
    /// Enforcement level of this criterion (from the last-iterated run;
    /// iteration order is filesystem-defined, not chronological).
    pub enforcement: Enforcement,
}

/// Completion velocity for a single milestone.
///
/// Measures how fast chunks are being completed relative to elapsed calendar time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneVelocity {
    /// Unique slug identifying the milestone.
    pub milestone_slug: String,
    /// Human-readable display name.
    pub milestone_name: String,
    /// Number of chunks marked as completed.
    pub chunks_completed: usize,
    /// Total number of chunks in the milestone.
    pub total_chunks: usize,
    /// Calendar days between milestone creation and its last update
    /// (`updated_at − created_at`). Clamped to `>= 0.0`.
    pub days_elapsed: f64,
    /// Completed chunks per calendar day
    /// (`chunks_completed / max(1, days_elapsed)`).
    pub chunks_per_day: f64,
}

/// Aggregated analytics report combining failure frequency and milestone velocity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsReport {
    /// Per-criterion failure frequency across all specs.
    pub failure_frequency: Vec<FailureFrequency>,
    /// Per-milestone completion velocity.
    pub milestone_velocity: Vec<MilestoneVelocity>,
    /// Number of history record files that could not be deserialized.
    pub unreadable_records: usize,
}

/// Compute per-criterion failure frequency from gate run history.
///
/// Scans `.assay/results/<spec>/` directories for JSON records, aggregating
/// pass/fail counts by `(spec_name, criterion_name)` pair. Records that
/// cannot be read or deserialized are counted in the returned `usize`
/// (unreadable count) and logged to stderr.
///
/// Returns `(frequencies, unreadable_count)`.
pub fn compute_failure_frequency(assay_dir: &Path) -> Result<(Vec<FailureFrequency>, usize)> {
    let results_dir = assay_dir.join("results");
    if !results_dir.is_dir() {
        return Ok((vec![], 0));
    }

    /// Accumulator for per-criterion aggregation.
    struct CriterionAcc {
        fail_count: usize,
        total_runs: usize,
        enforcement: Enforcement,
    }

    // Key: (spec_name, criterion_name)
    let mut agg: HashMap<(String, String), CriterionAcc> = HashMap::new();
    let mut unreadable: usize = 0;

    let spec_dirs = std::fs::read_dir(&results_dir)
        .map_err(|e| crate::error::AssayError::io("reading results directory", &results_dir, e))?;

    for spec_entry in spec_dirs {
        let spec_entry = match spec_entry {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "Skipping unreadable results entry");
                unreadable += 1;
                continue;
            }
        };
        let spec_path = spec_entry.path();
        if !spec_path.is_dir() {
            continue;
        }

        let run_files = match std::fs::read_dir(&spec_path) {
            Ok(rd) => rd,
            Err(e) => {
                warn!(
                    path = %spec_path.display(),
                    error = %e,
                    "Could not read spec directory — all history for this spec skipped"
                );
                unreadable += 1;
                continue;
            }
        };

        for run_entry in run_files {
            let run_entry = match run_entry {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "Skipping unreadable run entry");
                    unreadable += 1;
                    continue;
                }
            };
            let run_path = run_entry.path();
            if run_path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = match std::fs::read_to_string(&run_path) {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        path = %run_path.display(),
                        error = %e,
                        "Unreadable history record"
                    );
                    unreadable += 1;
                    continue;
                }
            };

            let record: assay_types::GateRunRecord = match serde_json::from_str(&content) {
                Ok(r) => r,
                Err(e) => {
                    warn!(
                        path = %run_path.display(),
                        error = %e,
                        "Unreadable history record"
                    );
                    unreadable += 1;
                    continue;
                }
            };

            let spec_name = &record.summary.spec_name;
            for cr in &record.summary.results {
                // Skip criteria that were not evaluated (descriptive-only).
                let Some(ref result) = cr.result else {
                    continue;
                };

                let key = (spec_name.clone(), cr.criterion_name.clone());
                let entry = agg.entry(key).or_insert(CriterionAcc {
                    fail_count: 0,
                    total_runs: 0,
                    enforcement: cr.enforcement,
                });
                entry.total_runs += 1;
                if !result.passed {
                    entry.fail_count += 1;
                }
                // Update enforcement to last-iterated value
                // (iteration order is filesystem-defined, not chronological).
                entry.enforcement = cr.enforcement;
            }
        }
    }

    let mut freqs: Vec<FailureFrequency> = agg
        .into_iter()
        .map(|((spec_name, criterion_name), acc)| FailureFrequency {
            spec_name,
            criterion_name,
            fail_count: acc.fail_count,
            total_runs: acc.total_runs,
            enforcement: acc.enforcement,
        })
        .collect();

    // Sort by fail_count desc, then spec_name asc, criterion_name asc.
    freqs.sort_by(|a, b| {
        b.fail_count
            .cmp(&a.fail_count)
            .then(a.spec_name.cmp(&b.spec_name))
            .then(a.criterion_name.cmp(&b.criterion_name))
    });

    Ok((freqs, unreadable))
}

/// Compute per-milestone completion velocity.
///
/// Scans `.assay/milestones/` for TOML files, computing `chunks_per_day`
/// for each milestone that has at least one completed chunk, regardless of status.
/// Milestones with zero completed chunks are excluded.
pub fn compute_milestone_velocity(assay_dir: &Path) -> Result<Vec<MilestoneVelocity>> {
    let milestones = match crate::milestone::milestone_scan(assay_dir) {
        Ok(m) => m,
        Err(e) => {
            // Only suppress when the milestones directory simply doesn't exist yet.
            let milestones_dir = assay_dir.join("milestones");
            if !milestones_dir.exists() {
                return Ok(vec![]);
            }
            // Any other error (permission denied, corrupt TOML, etc.) is unexpected.
            warn!(
                path = %milestones_dir.display(),
                error = %e,
                "Could not read milestones directory"
            );
            return Ok(vec![]);
        }
    };

    let mut velocities: Vec<MilestoneVelocity> = milestones
        .into_iter()
        .filter(|m| !m.completed_chunks.is_empty())
        .map(|m| {
            let chunks_completed = m.completed_chunks.len();
            let total_chunks = m.chunks.len();
            let raw_days = (m.updated_at - m.created_at).num_seconds() as f64 / 86400.0;
            if raw_days < 0.0 {
                warn!(
                    milestone_slug = %m.slug,
                    days_elapsed = raw_days,
                    "Milestone has updated_at before created_at — timestamps may be corrupt, treating as 0 days elapsed"
                );
            }
            let days_elapsed = raw_days.max(0.0);
            let effective_days = days_elapsed.max(1.0);
            let chunks_per_day = chunks_completed as f64 / effective_days;

            MilestoneVelocity {
                milestone_slug: m.slug,
                milestone_name: m.name,
                chunks_completed,
                total_chunks,
                days_elapsed,
                chunks_per_day,
            }
        })
        .collect();

    // Sort by chunks_per_day desc, then milestone_slug asc.
    velocities.sort_by(|a, b| {
        b.chunks_per_day
            .partial_cmp(&a.chunks_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.milestone_slug.cmp(&b.milestone_slug))
    });

    Ok(velocities)
}

/// Compute the full analytics report, combining failure frequency and milestone velocity.
///
/// This is the primary public entry point. It calls both
/// [`compute_failure_frequency`] and [`compute_milestone_velocity`],
/// composing their results into an [`AnalyticsReport`].
pub fn compute_analytics(assay_dir: &Path) -> Result<AnalyticsReport> {
    let (failure_frequency, unreadable_records) = compute_failure_frequency(assay_dir)?;
    let milestone_velocity = compute_milestone_velocity(assay_dir)?;

    Ok(AnalyticsReport {
        failure_frequency,
        milestone_velocity,
        unreadable_records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_results_returns_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        // No results dir exists at all.
        let (freqs, unreadable) = compute_failure_frequency(dir.path()).unwrap();
        assert!(freqs.is_empty());
        assert_eq!(unreadable, 0);
    }

    #[test]
    fn test_skipped_criterion_not_counted() {
        let dir = tempfile::TempDir::new().unwrap();
        let spec_dir = dir.path().join("results").join("spec-skip");
        std::fs::create_dir_all(&spec_dir).unwrap();

        // Build a record where one criterion has result: None (skipped).
        let ts = chrono::Utc::now();
        let record = assay_types::GateRunRecord {
            run_id: "20260101T000000Z-skip01".to_string(),
            assay_version: "0.0.0-test".to_string(),
            timestamp: ts,
            working_dir: None,
            diff_truncation: None,
            precondition_blocked: None,
            summary: assay_types::GateRunSummary {
                spec_name: "spec-skip".to_string(),
                results: vec![
                    assay_types::CriterionResult {
                        criterion_name: "evaluated".to_string(),
                        result: Some(assay_types::GateResult {
                            passed: true,
                            kind: assay_types::GateKind::Command {
                                cmd: "true".to_string(),
                            },
                            stdout: String::new(),
                            stderr: String::new(),
                            exit_code: Some(0),
                            duration_ms: 1,
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
                    },
                    assay_types::CriterionResult {
                        criterion_name: "skipped".to_string(),
                        result: None, // Skipped — should not be counted
                        enforcement: Enforcement::Advisory,
                        source: None,
                    },
                ],
                passed: 1,
                failed: 0,
                skipped: 1,
                total_duration_ms: 1,
                enforcement: assay_types::EnforcementSummary::default(),
            },
        };

        let json = serde_json::to_string_pretty(&record).unwrap();
        std::fs::write(spec_dir.join("20260101T000000Z-skip01.json"), json).unwrap();

        let (freqs, unreadable) = compute_failure_frequency(dir.path()).unwrap();
        assert_eq!(unreadable, 0);
        // Only the evaluated criterion should appear.
        assert_eq!(freqs.len(), 1);
        assert_eq!(freqs[0].criterion_name, "evaluated");
        assert_eq!(freqs[0].total_runs, 1);
        assert_eq!(freqs[0].fail_count, 0);
    }
}
