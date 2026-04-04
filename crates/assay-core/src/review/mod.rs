//! Spec review module — structural completeness checks.
//!
//! [`run_structural_review()`] evaluates a spec against 6 machine-checkable
//! structural checks and returns a [`ReviewReport`]. Checks that require a
//! `FeatureSpec` (spec.toml) are skipped when none is available.
//!
//! [`save_review()`] persists a `ReviewReport` as JSON under
//! `.assay/reviews/<spec>/<run-id>.json` using atomic writes.
//!
//! [`list_reviews()`] reads past review reports sorted by timestamp descending.

use std::io::Write;
use std::path::{Path, PathBuf};

use assay_types::GatesSpec;
use assay_types::feature_spec::{FeatureSpec, Obligation, SpecStatus};
use assay_types::review::{ReviewCheck, ReviewCheckKind, ReviewReport};
use chrono::Utc;
use tempfile::NamedTempFile;

use crate::error::{AssayError, Result};
use crate::history::{generate_run_id, validate_path_component};
use crate::spec::coverage::compute_coverage;
use crate::spec::is_valid_req_id;
use assay_types::CoverageReport;

/// Run all 6 structural checks against a spec.
///
/// Checks that require a `FeatureSpec` are skipped when `feature` is `None`.
/// The `spec_slug` identifies the spec for the report.
pub fn run_structural_review(
    spec_slug: &str,
    gates: &GatesSpec,
    feature: Option<&FeatureSpec>,
) -> ReviewReport {
    tracing::debug!(spec_slug, "running structural review");
    // Compute coverage once; reused by req-coverage and no-orphaned-criteria.
    let coverage = feature.map(|f| compute_coverage(spec_slug, gates, Some(f)));
    let checks = vec![
        check_req_coverage(coverage.as_ref()),
        check_acceptance_criteria(feature),
        check_req_id_format(feature),
        check_criterion_traceability(gates),
        check_no_orphaned_criteria(coverage.as_ref()),
        check_status_consistency(feature),
    ];

    // Compute summary counts based on the explicit `skipped` flag.
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for check in &checks {
        if check.skipped {
            skipped += 1;
        } else if check.passed {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    let report = ReviewReport {
        spec: spec_slug.to_string(),
        run_id: None, // Set by save_review() on persistence.
        timestamp: Utc::now(),
        checks,
        passed,
        failed,
        skipped,
    };
    tracing::info!(
        spec_slug,
        passed = report.passed,
        failed = report.failed,
        skipped = report.skipped,
        "structural review complete"
    );
    report
}

/// Persist a `ReviewReport` to `.assay/reviews/<spec>/<run-id>.json`.
///
/// The `run_id` is generated from the report's timestamp. The file is written
/// atomically via tempfile-then-rename to prevent corruption on crash.
///
/// Returns the path of the written file.
pub fn save_review(assay_dir: &Path, report: &ReviewReport) -> Result<PathBuf> {
    validate_path_component(&report.spec, "spec slug")?;

    let run_id = generate_run_id(&report.timestamp);
    // generate_run_id output is safe for filenames, but validate defensively to guard
    // against future changes to the ID format.
    validate_path_component(&run_id, "run id")?;
    let reviews_dir = assay_dir.join("reviews").join(&report.spec);
    std::fs::create_dir_all(&reviews_dir).map_err(|e| AssayError::Io {
        operation: "creating reviews directory".to_string(),
        path: reviews_dir.clone(),
        source: e,
    })?;

    let target = reviews_dir.join(format!("{run_id}.json"));
    let json = serde_json::to_string_pretty(&ReviewReport {
        run_id: Some(run_id.clone()),
        ..report.clone()
    })
    .map_err(|e| AssayError::Io {
        operation: "serializing ReviewReport".to_string(),
        path: target.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })?;

    let mut tmp = NamedTempFile::new_in(&reviews_dir).map_err(|e| AssayError::Io {
        operation: "creating tempfile for review".to_string(),
        path: reviews_dir.clone(),
        source: e,
    })?;
    tmp.write_all(json.as_bytes()).map_err(|e| AssayError::Io {
        operation: "writing review tempfile".to_string(),
        path: target.clone(),
        source: e,
    })?;
    tmp.as_file().sync_all().map_err(|e| AssayError::Io {
        operation: "syncing review tempfile".to_string(),
        path: target.clone(),
        source: e,
    })?;
    tmp.persist(&target).map_err(|e| AssayError::Io {
        operation: "persisting review file".to_string(),
        path: target.clone(),
        source: e.error,
    })?;
    tracing::info!(
        spec = %report.spec,
        run_id = %run_id,
        path = %target.display(),
        "review saved"
    );
    Ok(target)
}

/// List past review reports for a spec, sorted by timestamp descending.
///
/// Returns an empty `Vec` if the reviews directory does not exist.
pub fn list_reviews(assay_dir: &Path, spec_slug: &str) -> Result<Vec<ReviewReport>> {
    validate_path_component(spec_slug, "spec slug")?;

    let reviews_dir = assay_dir.join("reviews").join(spec_slug);
    if !reviews_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut reports = Vec::new();
    let entries = std::fs::read_dir(&reviews_dir).map_err(|e| AssayError::Io {
        operation: "reading reviews directory".to_string(),
        path: reviews_dir.clone(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| AssayError::Io {
            operation: "iterating reviews directory".to_string(),
            path: reviews_dir.clone(),
            source: e,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let content = std::fs::read_to_string(&path).map_err(|e| AssayError::Io {
                operation: "reading review file".to_string(),
                path: path.clone(),
                source: e,
            })?;
            match serde_json::from_str::<ReviewReport>(&content) {
                Ok(report) => reports.push(report),
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skipping unreadable review file");
                }
            }
        }
    }

    // Sort by timestamp descending (most recent first); tiebreak on run_id for stability.
    reports.sort_by(|a, b| {
        b.timestamp
            .cmp(&a.timestamp)
            .then_with(|| b.run_id.cmp(&a.run_id))
    });
    Ok(reports)
}

/// Check 1: req-coverage — all declared requirements have at least one criterion.
///
/// Accepts a pre-computed `CoverageReport` (or `None` when there is no `spec.toml`).
fn check_req_coverage(coverage: Option<&CoverageReport>) -> ReviewCheck {
    let name = "req-coverage".to_string();
    let kind = ReviewCheckKind::Structural;

    let Some(report) = coverage else {
        return ReviewCheck {
            name,
            kind,
            skipped: true,
            passed: true,
            message: "skipped — no spec.toml".to_string(),
            details: None,
        };
    };

    if report.uncovered.is_empty() {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: format!(
                "all {} requirements covered by criteria",
                report.total_requirements
            ),
            details: None,
        }
    } else {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: false,
            message: format!(
                "{} of {} requirements uncovered",
                report.uncovered.len(),
                report.total_requirements
            ),
            details: Some(format!("uncovered: {}", report.uncovered.join(", "))),
        }
    }
}

/// Check 2: acceptance-criteria — every SHALL requirement has ≥1 acceptance criterion.
fn check_acceptance_criteria(feature: Option<&FeatureSpec>) -> ReviewCheck {
    let name = "acceptance-criteria".to_string();
    let kind = ReviewCheckKind::Structural;

    let Some(feature) = feature else {
        return ReviewCheck {
            name,
            kind,
            skipped: true,
            passed: true,
            message: "skipped — no spec.toml".to_string(),
            details: None,
        };
    };

    let shall_reqs: Vec<&str> = feature
        .requirements
        .iter()
        .filter(|r| r.obligation == Obligation::Shall)
        .filter(|r| r.acceptance_criteria.is_empty())
        .map(|r| r.id.as_str())
        .collect();

    if shall_reqs.is_empty() {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: "all SHALL requirements have acceptance criteria".to_string(),
            details: None,
        }
    } else {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: false,
            message: format!(
                "{} SHALL requirement(s) missing acceptance criteria",
                shall_reqs.len()
            ),
            details: Some(format!("missing: {}", shall_reqs.join(", "))),
        }
    }
}

/// Check 3: req-id-format — all requirement IDs match REQ-AREA-NNN pattern.
fn check_req_id_format(feature: Option<&FeatureSpec>) -> ReviewCheck {
    let name = "req-id-format".to_string();
    let kind = ReviewCheckKind::Structural;

    let Some(feature) = feature else {
        return ReviewCheck {
            name,
            kind,
            skipped: true,
            passed: true,
            message: "skipped — no spec.toml".to_string(),
            details: None,
        };
    };

    let invalid: Vec<&str> = feature
        .requirements
        .iter()
        .filter(|r| !is_valid_req_id(&r.id))
        .map(|r| r.id.as_str())
        .collect();

    if invalid.is_empty() {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: format!(
                "all {} requirement IDs follow REQ-AREA-NNN format",
                feature.requirements.len()
            ),
            details: None,
        }
    } else {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: false,
            message: format!(
                "{} requirement ID(s) do not follow REQ-AREA-NNN format",
                invalid.len()
            ),
            details: Some(format!("invalid: {}", invalid.join(", "))),
        }
    }
}

/// Check 4: criterion-traceability — criteria should have requirements references.
///
/// Fails if >50% of criteria have no `requirements` field (D008).
/// Below threshold, passes with an advisory message.
fn check_criterion_traceability(gates: &GatesSpec) -> ReviewCheck {
    let name = "criterion-traceability".to_string();
    let kind = ReviewCheckKind::Structural;

    let total = gates.criteria.len();
    if total == 0 {
        return ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: "no criteria to check".to_string(),
            details: None,
        };
    }

    let without_reqs = gates
        .criteria
        .iter()
        .filter(|c| c.requirements.is_empty())
        .count();

    let pct = without_reqs as f64 / total as f64 * 100.0;

    if without_reqs == 0 {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: format!("all {total} criteria have requirements traceability"),
            details: None,
        }
    } else if pct > 50.0 {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: false,
            message: format!(
                "{without_reqs} of {total} criteria ({pct:.0}%) lack requirements — exceeds 50% threshold"
            ),
            details: Some(format!(
                "criteria without requirements: {}",
                gates
                    .criteria
                    .iter()
                    .filter(|c| c.requirements.is_empty())
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    } else {
        // Advisory: passes but warns.
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: format!(
                "{without_reqs} of {total} criteria ({pct:.0}%) lack requirements — advisory (below 50% threshold)"
            ),
            details: Some(format!(
                "criteria without requirements: {}",
                gates
                    .criteria
                    .iter()
                    .filter(|c| c.requirements.is_empty())
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

/// Check 5: no-orphaned-criteria — no criterion references an unknown REQ-ID.
///
/// Accepts a pre-computed `CoverageReport` (or `None` when there is no `spec.toml`).
fn check_no_orphaned_criteria(coverage: Option<&CoverageReport>) -> ReviewCheck {
    let name = "no-orphaned-criteria".to_string();
    let kind = ReviewCheckKind::Structural;

    let Some(report) = coverage else {
        return ReviewCheck {
            name,
            kind,
            skipped: true,
            passed: true,
            message: "skipped — no spec.toml".to_string(),
            details: None,
        };
    };

    if report.orphaned.is_empty() {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: "no orphaned criterion requirement references".to_string(),
            details: None,
        }
    } else {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: false,
            message: format!(
                "{} criterion requirement reference(s) point to unknown REQ-IDs",
                report.orphaned.len()
            ),
            details: Some(format!("orphaned: {}", report.orphaned.join(", "))),
        }
    }
}

/// Check 6: status-consistency — if spec status is Verified, all requirements
/// should also be Verified.
fn check_status_consistency(feature: Option<&FeatureSpec>) -> ReviewCheck {
    let name = "status-consistency".to_string();
    let kind = ReviewCheckKind::Structural;

    let Some(feature) = feature else {
        return ReviewCheck {
            name,
            kind,
            skipped: true,
            passed: true,
            message: "skipped — no spec.toml".to_string(),
            details: None,
        };
    };

    if feature.status != SpecStatus::Verified {
        return ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: format!(
                "spec status is {} (not Verified) — consistency check not applicable",
                feature.status
            ),
            details: None,
        };
    }

    let non_verified: Vec<&str> = feature
        .requirements
        .iter()
        .filter(|r| r.status != SpecStatus::Verified)
        .map(|r| r.id.as_str())
        .collect();

    if non_verified.is_empty() {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: true,
            message: "spec is Verified and all requirements are Verified".to_string(),
            details: None,
        }
    } else {
        ReviewCheck {
            name,
            kind,
            skipped: false,
            passed: false,
            message: format!(
                "spec is Verified but {} requirement(s) are not",
                non_verified.len()
            ),
            details: Some(format!("non-verified: {}", non_verified.join(", "))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::coverage::compute_coverage;
    use assay_types::Criterion;
    use assay_types::feature_spec::{
        AcceptanceCriterion, AcceptanceCriterionType, Obligation, Priority, Requirement,
    };

    // ── Test helpers ─────────────────────────────────────────────────

    fn make_requirement(id: &str) -> Requirement {
        Requirement {
            id: id.to_string(),
            title: format!("Requirement {id}"),
            statement: "Test statement".to_string(),
            rationale: String::new(),
            obligation: Obligation::Shall,
            priority: Priority::Must,
            verification: Default::default(),
            status: Default::default(),
            acceptance_criteria: vec![],
        }
    }

    fn make_requirement_with_ac(id: &str) -> Requirement {
        let mut req = make_requirement(id);
        req.acceptance_criteria = vec![AcceptanceCriterion {
            criterion: "Given X, when Y, then Z".to_string(),
            criterion_type: AcceptanceCriterionType::default(),
        }];
        req
    }

    fn make_requirement_with_obligation(id: &str, obligation: Obligation) -> Requirement {
        let mut req = make_requirement(id);
        req.obligation = obligation;
        req
    }

    fn make_criterion(name: &str, reqs: &[&str]) -> Criterion {
        Criterion {
            name: name.to_string(),
            description: "Test criterion".to_string(),
            cmd: Some("echo ok".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: reqs.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_gates(criteria: Vec<Criterion>) -> GatesSpec {
        GatesSpec {
            name: "test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            milestone: None,
            order: None,
            criteria,
        }
    }

    fn make_feature(requirements: Vec<Requirement>) -> FeatureSpec {
        FeatureSpec {
            name: "test".to_string(),
            status: Default::default(),
            version: String::new(),
            overview: None,
            constraints: None,
            users: vec![],
            requirements,
            quality: None,
            assumptions: vec![],
            dependencies: vec![],
            risks: vec![],
            verification: None,
        }
    }

    // ── Check 1: req-coverage ────────────────────────────────────────

    #[test]
    fn req_coverage_pass_all_covered() {
        let feature = make_feature(vec![make_requirement("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);
        let cov = compute_coverage("test", &gates, Some(&feature));

        let check = check_req_coverage(Some(&cov));
        assert!(check.passed);
        assert!(!check.skipped);
    }

    #[test]
    fn req_coverage_fail_uncovered() {
        let feature = make_feature(vec![
            make_requirement("REQ-AUTH-001"),
            make_requirement("REQ-AUTH-002"),
        ]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);
        let cov = compute_coverage("test", &gates, Some(&feature));

        let check = check_req_coverage(Some(&cov));
        assert!(!check.passed);
        assert!(check.details.as_ref().unwrap().contains("REQ-AUTH-002"));
    }

    #[test]
    fn req_coverage_skip_no_feature() {
        let check = check_req_coverage(None);
        assert!(check.passed);
        assert!(check.skipped);
    }

    // ── Check 2: acceptance-criteria ─────────────────────────────────

    #[test]
    fn acceptance_criteria_pass_all_have_ac() {
        let feature = make_feature(vec![make_requirement_with_ac("REQ-AUTH-001")]);
        let check = check_acceptance_criteria(Some(&feature));
        assert!(check.passed);
        assert!(!check.skipped);
    }

    #[test]
    fn acceptance_criteria_fail_missing_ac() {
        let feature = make_feature(vec![
            make_requirement_with_ac("REQ-AUTH-001"),
            make_requirement("REQ-AUTH-002"), // no AC
        ]);
        let check = check_acceptance_criteria(Some(&feature));
        assert!(!check.passed);
        assert!(check.details.as_ref().unwrap().contains("REQ-AUTH-002"));
    }

    #[test]
    fn acceptance_criteria_skip_no_feature() {
        let check = check_acceptance_criteria(None);
        assert!(check.passed);
        assert!(check.skipped);
    }

    #[test]
    fn acceptance_criteria_should_reqs_ignored() {
        // SHOULD requirements without AC should not fail the check.
        let feature = make_feature(vec![make_requirement_with_obligation(
            "REQ-AUTH-001",
            Obligation::Should,
        )]);
        let check = check_acceptance_criteria(Some(&feature));
        assert!(check.passed);
    }

    // ── Check 3: req-id-format ───────────────────────────────────────

    #[test]
    fn req_id_format_pass_valid() {
        let feature = make_feature(vec![
            make_requirement("REQ-AUTH-001"),
            make_requirement("REQ-SEC-42"),
        ]);
        let check = check_req_id_format(Some(&feature));
        assert!(check.passed);
    }

    #[test]
    fn req_id_format_fail_invalid() {
        let mut req = make_requirement("bad-id-format");
        req.id = "bad-id-format".to_string();
        let feature = make_feature(vec![req]);
        let check = check_req_id_format(Some(&feature));
        assert!(!check.passed);
        assert!(check.details.as_ref().unwrap().contains("bad-id-format"));
    }

    #[test]
    fn req_id_format_skip_no_feature() {
        let check = check_req_id_format(None);
        assert!(check.passed);
        assert!(check.skipped);
    }

    // ── Check 4: criterion-traceability ──────────────────────────────

    #[test]
    fn criterion_traceability_pass_all_have_reqs() {
        let gates = make_gates(vec![
            make_criterion("c1", &["REQ-AUTH-001"]),
            make_criterion("c2", &["REQ-AUTH-002"]),
        ]);
        let check = check_criterion_traceability(&gates);
        assert!(check.passed);
        assert!(check.message.contains("all"));
    }

    #[test]
    fn criterion_traceability_fail_majority_missing() {
        let gates = make_gates(vec![
            make_criterion("c1", &[]),
            make_criterion("c2", &[]),
            make_criterion("c3", &["REQ-AUTH-001"]),
        ]);
        // 2/3 = 67% > 50% → fail
        let check = check_criterion_traceability(&gates);
        assert!(!check.passed);
        assert!(check.message.contains("67%"));
    }

    #[test]
    fn criterion_traceability_advisory_below_threshold() {
        let gates = make_gates(vec![
            make_criterion("c1", &[]),
            make_criterion("c2", &["REQ-AUTH-001"]),
            make_criterion("c3", &["REQ-AUTH-002"]),
        ]);
        // 1/3 = 33% < 50% → advisory (passes with warning)
        let check = check_criterion_traceability(&gates);
        assert!(check.passed);
        assert!(check.message.contains("advisory"));
    }

    #[test]
    fn criterion_traceability_empty_criteria() {
        let gates = make_gates(vec![]);
        let check = check_criterion_traceability(&gates);
        assert!(check.passed);
    }

    // ── Check 5: no-orphaned-criteria ────────────────────────────────

    #[test]
    fn no_orphaned_criteria_pass() {
        let feature = make_feature(vec![make_requirement("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);
        let cov = compute_coverage("test", &gates, Some(&feature));
        let check = check_no_orphaned_criteria(Some(&cov));
        assert!(check.passed);
    }

    #[test]
    fn no_orphaned_criteria_fail() {
        let feature = make_feature(vec![make_requirement("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion(
            "c1",
            &["REQ-AUTH-001", "REQ-GHOST-999"],
        )]);
        let cov = compute_coverage("test", &gates, Some(&feature));
        let check = check_no_orphaned_criteria(Some(&cov));
        assert!(!check.passed);
        assert!(check.details.as_ref().unwrap().contains("REQ-GHOST-999"));
    }

    #[test]
    fn no_orphaned_criteria_skip_no_feature() {
        let check = check_no_orphaned_criteria(None);
        assert!(check.passed);
        assert!(check.skipped);
    }

    // ── Check 6: status-consistency ──────────────────────────────────

    #[test]
    fn status_consistency_pass_all_verified() {
        let mut feature = make_feature(vec![make_requirement("REQ-AUTH-001")]);
        feature.status = SpecStatus::Verified;
        feature.requirements[0].status = SpecStatus::Verified;

        let check = check_status_consistency(Some(&feature));
        assert!(check.passed);
    }

    #[test]
    fn status_consistency_fail_req_not_verified() {
        let mut feature = make_feature(vec![
            make_requirement("REQ-AUTH-001"),
            make_requirement("REQ-AUTH-002"),
        ]);
        feature.status = SpecStatus::Verified;
        feature.requirements[0].status = SpecStatus::Verified;
        // requirements[1] is still Draft

        let check = check_status_consistency(Some(&feature));
        assert!(!check.passed);
        assert!(check.details.as_ref().unwrap().contains("REQ-AUTH-002"));
    }

    #[test]
    fn status_consistency_pass_not_verified_spec() {
        let feature = make_feature(vec![make_requirement("REQ-AUTH-001")]);
        // status is Draft → check not applicable → passes
        let check = check_status_consistency(Some(&feature));
        assert!(check.passed);
        assert!(check.message.contains("not Verified"));
    }

    #[test]
    fn status_consistency_skip_no_feature() {
        let check = check_status_consistency(None);
        assert!(check.passed);
        assert!(check.skipped);
    }

    // ── Integration: run_structural_review ────────────────────────────

    #[test]
    fn run_review_all_pass() {
        let feature = make_feature(vec![make_requirement_with_ac("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);

        let report = run_structural_review("test", &gates, Some(&feature));
        assert_eq!(report.checks.len(), 6);
        assert_eq!(report.failed, 0);
        assert_eq!(report.skipped, 0);
        // passed = 6 (all pass including traceability which has 1 criterion with reqs)
        assert_eq!(report.passed, 6);
    }

    #[test]
    fn run_review_no_feature_all_skip() {
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);
        let report = run_structural_review("test", &gates, None);

        assert_eq!(report.checks.len(), 6);
        // 5 checks skip (req-coverage, acceptance-criteria, req-id-format, no-orphaned-criteria, status-consistency)
        // 1 check runs (criterion-traceability — doesn't require feature)
        assert_eq!(report.skipped, 5);
    }

    #[test]
    fn run_review_counts_correct() {
        let feature = make_feature(vec![
            make_requirement("REQ-AUTH-001"),
            make_requirement("REQ-AUTH-002"),
        ]);
        // c1 covers REQ-AUTH-001 but REQ-AUTH-002 is uncovered → req-coverage fails
        // both lack acceptance criteria → acceptance-criteria fails
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);

        let report = run_structural_review("test", &gates, Some(&feature));
        assert_eq!(report.checks.len(), 6);
        assert_eq!(
            report.failed, 2,
            "expected exactly req-coverage + acceptance-criteria to fail"
        );
        assert_eq!(report.passed + report.failed + report.skipped, 6);
    }

    #[test]
    fn review_report_serde_roundtrip() {
        let feature = make_feature(vec![make_requirement_with_ac("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);

        let report = run_structural_review("test", &gates, Some(&feature));
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ReviewReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report.spec, deserialized.spec);
        assert_eq!(report.checks.len(), deserialized.checks.len());
        assert_eq!(report.passed, deserialized.passed);
        assert_eq!(report.failed, deserialized.failed);
        assert_eq!(report.skipped, deserialized.skipped);
    }

    // ── Persistence: save_review ──────────────────────────────────────

    #[test]
    fn save_review_creates_file_with_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let assay_dir = dir.path();

        let feature = make_feature(vec![make_requirement_with_ac("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);
        let report = run_structural_review("my-spec", &gates, Some(&feature));

        let path = save_review(assay_dir, &report).unwrap();
        assert!(path.exists());
        assert!(path.extension().unwrap() == "json");

        // Verify the file contains valid JSON that deserializes back.
        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: ReviewReport = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.spec, "my-spec");
        assert!(loaded.run_id.is_some());
        assert!(!loaded.run_id.as_ref().unwrap().is_empty());
        assert_eq!(loaded.checks.len(), 6);
    }

    #[test]
    fn save_review_validates_spec_slug() {
        let dir = tempfile::tempdir().unwrap();
        let assay_dir = dir.path();

        let mut report = run_structural_review("valid", &make_gates(vec![]), None);
        report.spec = "../escape".to_string();

        let result = save_review(assay_dir, &report);
        assert!(result.is_err());
    }

    // ── Persistence: list_reviews ─────────────────────────────────────

    #[test]
    fn list_reviews_returns_sorted_descending() {
        let dir = tempfile::tempdir().unwrap();
        let assay_dir = dir.path();

        let feature = make_feature(vec![make_requirement_with_ac("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);

        // Save two reviews with a small time gap.
        let report1 = run_structural_review("my-spec", &gates, Some(&feature));
        save_review(assay_dir, &report1).unwrap();

        // Small delay so timestamps differ.
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report2 = run_structural_review("my-spec", &gates, Some(&feature));
        save_review(assay_dir, &report2).unwrap();

        let reviews = list_reviews(assay_dir, "my-spec").unwrap();
        assert_eq!(reviews.len(), 2);
        // Most recent first.
        // Timestamps must be strictly ordered (generate_run_id randomises subsec_nanos).
        assert!(
            reviews[0].timestamp > reviews[1].timestamp,
            "expected most-recent review first, but timestamps were equal or reversed: {:?} vs {:?}",
            reviews[0].timestamp,
            reviews[1].timestamp
        );
    }

    #[test]
    fn list_reviews_empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let reviews = list_reviews(dir.path(), "nonexistent").unwrap();
        assert!(reviews.is_empty());
    }

    #[test]
    fn list_reviews_validates_slug() {
        let dir = tempfile::tempdir().unwrap();
        let result = list_reviews(dir.path(), "../escape");
        assert!(result.is_err());
    }

    #[test]
    fn list_reviews_skips_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        let assay_dir = dir.path();

        let feature = make_feature(vec![make_requirement_with_ac("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);
        let report = run_structural_review("my-spec", &gates, Some(&feature));
        save_review(assay_dir, &report).unwrap();

        // Inject a non-JSON file into the reviews directory.
        let reviews_dir = assay_dir.join("reviews").join("my-spec");
        std::fs::write(reviews_dir.join("notes.txt"), "not json").unwrap();
        std::fs::write(reviews_dir.join("corrupt.json"), "{not valid json").unwrap();

        // Should return only the 1 valid review and not error.
        let reviews = list_reviews(assay_dir, "my-spec").unwrap();
        assert_eq!(
            reviews.len(),
            1,
            "non-JSON and corrupt files should be skipped"
        );
    }

    #[test]
    fn criterion_traceability_exactly_50_pct_is_advisory() {
        // 2 of 4 criteria have no requirements → exactly 50% → advisory (not fail, per D008)
        let gates = make_gates(vec![
            make_criterion("c1", &[]),
            make_criterion("c2", &[]),
            make_criterion("c3", &["REQ-AUTH-001"]),
            make_criterion("c4", &["REQ-AUTH-002"]),
        ]);
        let check = check_criterion_traceability(&gates);
        assert!(
            check.passed,
            "exactly 50% without requirements should be advisory (threshold is >50%)"
        );
        assert!(check.message.contains("advisory"));
    }

    #[test]
    fn criterion_traceability_just_over_50_pct_fails() {
        // 3 of 5 = 60% → just over threshold → fail
        let gates = make_gates(vec![
            make_criterion("c1", &[]),
            make_criterion("c2", &[]),
            make_criterion("c3", &[]),
            make_criterion("c4", &["REQ-AUTH-001"]),
            make_criterion("c5", &["REQ-AUTH-002"]),
        ]);
        let check = check_criterion_traceability(&gates);
        assert!(
            !check.passed,
            "60% without requirements should fail (>50% threshold)"
        );
    }
}
