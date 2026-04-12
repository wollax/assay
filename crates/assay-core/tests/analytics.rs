//! Integration tests for `assay_core::history::analytics`.
//!
//! Each test creates synthetic gate run records and/or milestone TOML files
//! in a temp directory, then exercises the analytics compute functions against
//! that data.

use chrono::{DateTime, Duration, Utc};
use tempfile::TempDir;

use assay_types::{
    ChunkRef, CriterionResult, Enforcement, EnforcementSummary, GateKind, GateResult,
    GateRunRecord, GateRunSummary, Milestone, MilestoneStatus,
};

use assay_core::history;
use assay_core::history::analytics::{
    compute_analytics, compute_failure_frequency, compute_milestone_velocity,
};
use assay_core::milestone::milestone_save;

/// Counter for generating unique run IDs in tests.
static RUN_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Generate a unique run ID for tests (external crate can't use `generate_run_id`).
fn test_run_id() -> String {
    let n = RUN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let ts = Utc::now().format("%Y%m%dT%H%M%SZ");
    format!("{ts}-{n:06x}")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create and save a synthetic gate run record with the given criteria results.
///
/// `criteria` is a list of `(criterion_name, passed, enforcement)` tuples.
/// Each criterion is recorded as a `Command` kind result.
fn create_synthetic_record(
    assay_dir: &std::path::Path,
    spec_name: &str,
    criteria: Vec<(&str, bool, Enforcement)>,
) {
    let ts = Utc::now();
    let run_id = test_run_id();

    let results: Vec<CriterionResult> = criteria
        .into_iter()
        .map(|(name, passed, enforcement)| CriterionResult {
            criterion_name: name.to_string(),
            result: Some(GateResult {
                passed,
                kind: GateKind::Command {
                    cmd: format!("test-{name}"),
                },
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(if passed { 0 } else { 1 }),
                duration_ms: 10,
                timestamp: ts,
                truncated: false,
                original_bytes: None,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement,
            source: None,
        })
        .collect();

    let passed = results
        .iter()
        .filter(|r| r.result.as_ref().unwrap().passed)
        .count();
    let failed = results.len() - passed;

    let record = GateRunRecord {
        run_id,
        assay_version: "0.0.0-test".to_string(),
        timestamp: ts,
        working_dir: None,
        diff_truncation: None,
        summary: GateRunSummary {
            spec_name: spec_name.to_string(),
            results,
            passed,
            failed,
            skipped: 0,
            total_duration_ms: 10,
            enforcement: EnforcementSummary::default(),
        },
    };

    history::save(assay_dir, &record, None).expect("save synthetic record");
}

/// Create and save a synthetic milestone TOML file.
///
/// `chunks` is the total chunk count; `completed_chunks` is the number of those
/// chunks marked complete (generated as `chunk-0`..`chunk-{n-1}`). `created_at`
/// and `updated_at` control the velocity calculation.
#[allow(clippy::too_many_arguments)]
fn create_synthetic_milestone(
    assay_dir: &std::path::Path,
    slug: &str,
    name: &str,
    chunks: usize,
    completed_chunks: usize,
    status: MilestoneStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) {
    let chunk_refs: Vec<ChunkRef> = (0..chunks)
        .map(|i| ChunkRef {
            slug: format!("chunk-{i}"),
            order: i as u32,
            depends_on: vec![],
        })
        .collect();

    let completed: Vec<String> = (0..completed_chunks)
        .map(|i| format!("chunk-{i}"))
        .collect();

    let milestone = Milestone {
        slug: slug.to_string(),
        name: name.to_string(),
        description: None,
        status,
        chunks: chunk_refs,
        completed_chunks: completed,
        depends_on: vec![],
        pr_branch: None,
        pr_base: None,
        pr_number: None,
        pr_url: None,
        pr_labels: None,
        pr_reviewers: None,
        pr_body_template: None,
        created_at,
        updated_at,
    };

    milestone_save(assay_dir, &milestone).expect("save synthetic milestone");
}

// ---------------------------------------------------------------------------
// Tests: empty / edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_analytics_empty_results_dir() {
    let dir = TempDir::new().unwrap();
    // No results dir at all.
    let report = compute_analytics(dir.path()).unwrap();

    assert!(report.failure_frequency.is_empty());
    assert!(report.milestone_velocity.is_empty());
    assert_eq!(report.unreadable_records, 0);
}

// ---------------------------------------------------------------------------
// Tests: failure frequency
// ---------------------------------------------------------------------------

#[test]
fn test_failure_frequency_single_spec() {
    let dir = TempDir::new().unwrap();

    // Run 1: criterion-a passes, criterion-b fails
    create_synthetic_record(
        dir.path(),
        "spec-alpha",
        vec![
            ("criterion-a", true, Enforcement::Required),
            ("criterion-b", false, Enforcement::Advisory),
        ],
    );
    // Run 2: both pass
    create_synthetic_record(
        dir.path(),
        "spec-alpha",
        vec![
            ("criterion-a", true, Enforcement::Required),
            ("criterion-b", true, Enforcement::Advisory),
        ],
    );
    // Run 3: both fail
    create_synthetic_record(
        dir.path(),
        "spec-alpha",
        vec![
            ("criterion-a", false, Enforcement::Required),
            ("criterion-b", false, Enforcement::Advisory),
        ],
    );

    let (freqs, unreadable) = compute_failure_frequency(dir.path()).unwrap();

    assert_eq!(unreadable, 0);
    assert_eq!(freqs.len(), 2, "should have 2 criteria");

    let a = freqs
        .iter()
        .find(|f| f.criterion_name == "criterion-a")
        .unwrap();
    assert_eq!(a.spec_name, "spec-alpha");
    assert_eq!(a.total_runs, 3);
    assert_eq!(a.fail_count, 1); // failed in run 3

    let b = freqs
        .iter()
        .find(|f| f.criterion_name == "criterion-b")
        .unwrap();
    assert_eq!(b.spec_name, "spec-alpha");
    assert_eq!(b.total_runs, 3);
    assert_eq!(b.fail_count, 2); // failed in runs 1 and 3
}

#[test]
fn test_failure_frequency_multi_spec() {
    let dir = TempDir::new().unwrap();

    // Two different specs with a criterion that has the same name.
    create_synthetic_record(
        dir.path(),
        "spec-one",
        vec![("lint", false, Enforcement::Required)],
    );
    create_synthetic_record(
        dir.path(),
        "spec-two",
        vec![("lint", true, Enforcement::Required)],
    );

    let (freqs, unreadable) = compute_failure_frequency(dir.path()).unwrap();

    assert_eq!(unreadable, 0);
    assert_eq!(
        freqs.len(),
        2,
        "same criterion name in different specs → 2 entries"
    );

    let one = freqs.iter().find(|f| f.spec_name == "spec-one").unwrap();
    assert_eq!(one.criterion_name, "lint");
    assert_eq!(one.fail_count, 1);
    assert_eq!(one.total_runs, 1);

    let two = freqs.iter().find(|f| f.spec_name == "spec-two").unwrap();
    assert_eq!(two.criterion_name, "lint");
    assert_eq!(two.fail_count, 0);
    assert_eq!(two.total_runs, 1);
}

#[test]
fn test_failure_frequency_skips_corrupt_records() {
    let dir = TempDir::new().unwrap();

    // One valid record.
    create_synthetic_record(
        dir.path(),
        "spec-corrupt",
        vec![("check", true, Enforcement::Required)],
    );

    // One corrupt file in the same spec dir.
    let corrupt_path = dir
        .path()
        .join("results")
        .join("spec-corrupt")
        .join("20260101T000000Z-badbad.json");
    std::fs::write(&corrupt_path, "this is not valid JSON {{{").unwrap();

    let (freqs, unreadable) = compute_failure_frequency(dir.path()).unwrap();

    assert_eq!(
        unreadable, 1,
        "corrupt file should be counted as unreadable"
    );
    assert_eq!(freqs.len(), 1);
    assert_eq!(freqs[0].criterion_name, "check");
    assert_eq!(freqs[0].total_runs, 1);
}

// ---------------------------------------------------------------------------
// Tests: milestone velocity
// ---------------------------------------------------------------------------

#[test]
fn test_milestone_velocity_basic() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    let ten_days_ago = now - Duration::days(10);

    create_synthetic_milestone(
        dir.path(),
        "feature-x",
        "Feature X",
        5, // total chunks
        3, // completed chunks
        MilestoneStatus::InProgress,
        ten_days_ago,
        now,
    );

    let velocities = compute_milestone_velocity(dir.path()).unwrap();

    assert_eq!(velocities.len(), 1);
    let v = &velocities[0];
    assert_eq!(v.milestone_slug, "feature-x");
    assert_eq!(v.milestone_name, "Feature X");
    assert_eq!(v.chunks_completed, 3);
    assert_eq!(v.total_chunks, 5);
    assert!(
        v.days_elapsed >= 9.9,
        "days_elapsed should be ~10, got {}",
        v.days_elapsed
    );
    // chunks_per_day ≈ 3/10 = 0.3
    assert!(
        (v.chunks_per_day - 0.3).abs() < 0.05,
        "chunks_per_day should be ~0.3, got {}",
        v.chunks_per_day
    );
}

#[test]
fn test_milestone_velocity_zero_elapsed() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();

    // Created and updated same instant → days_elapsed ≈ 0 → uses max(1, days).
    create_synthetic_milestone(
        dir.path(),
        "just-started",
        "Just Started",
        4,
        2,
        MilestoneStatus::InProgress,
        now,
        now,
    );

    let velocities = compute_milestone_velocity(dir.path()).unwrap();

    assert_eq!(velocities.len(), 1);
    let v = &velocities[0];
    assert_eq!(v.chunks_completed, 2);
    // days_elapsed < 1 → effective_days = 1 → chunks_per_day = 2.0
    assert!(
        (v.chunks_per_day - 2.0).abs() < 0.1,
        "chunks_per_day should be ~2.0, got {}",
        v.chunks_per_day
    );
    // Should not panic or produce infinity/NaN.
    assert!(v.chunks_per_day.is_finite());
}

#[test]
fn test_milestone_velocity_excludes_zero_chunk_milestones() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();
    let five_days_ago = now - Duration::days(5);

    // Milestone with 0 completed chunks → excluded (regardless of status).
    create_synthetic_milestone(
        dir.path(),
        "draft-ms",
        "Draft Milestone",
        3,
        0,
        MilestoneStatus::Draft,
        five_days_ago,
        now,
    );

    // Milestone with completed chunks → included.
    create_synthetic_milestone(
        dir.path(),
        "active-ms",
        "Active Milestone",
        4,
        2,
        MilestoneStatus::InProgress,
        five_days_ago,
        now,
    );

    let velocities = compute_milestone_velocity(dir.path()).unwrap();

    assert_eq!(
        velocities.len(),
        1,
        "milestone with 0 completed chunks should be excluded"
    );
    assert_eq!(velocities[0].milestone_slug, "active-ms");
}

// ---------------------------------------------------------------------------
// Tests: composite
// ---------------------------------------------------------------------------

#[test]
fn test_compute_analytics_composes_both() {
    let dir = TempDir::new().unwrap();
    let now = Utc::now();

    // Create some history records.
    create_synthetic_record(
        dir.path(),
        "my-spec",
        vec![("build", true, Enforcement::Required)],
    );

    // Create a milestone.
    create_synthetic_milestone(
        dir.path(),
        "my-ms",
        "My Milestone",
        3,
        1,
        MilestoneStatus::InProgress,
        now - Duration::days(5),
        now,
    );

    let report = compute_analytics(dir.path()).unwrap();

    assert!(
        !report.failure_frequency.is_empty(),
        "should have frequency data"
    );
    assert!(
        !report.milestone_velocity.is_empty(),
        "should have velocity data"
    );
    assert_eq!(report.unreadable_records, 0);

    // Verify frequency data.
    assert_eq!(report.failure_frequency[0].spec_name, "my-spec");
    assert_eq!(report.failure_frequency[0].criterion_name, "build");

    // Verify velocity data.
    assert_eq!(report.milestone_velocity[0].milestone_slug, "my-ms");
    assert_eq!(report.milestone_velocity[0].chunks_completed, 1);
}
