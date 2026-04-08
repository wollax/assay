//! Requirements coverage analysis.
//!
//! Cross-references `FeatureSpec.requirements[].id` against
//! `GatesSpec.criteria[].requirements[]` to produce a [`CoverageReport`].

use std::collections::BTreeSet;

use assay_types::{CoverageReport, FeatureSpec, GatesSpec};

/// Compute a requirements coverage report.
///
/// Pure function — no IO. Receives already-loaded spec types.
///
/// When `feature` is `None` (no `spec.toml`), returns a report with zero
/// requirements and 100% coverage (nothing to miss — D007).
pub fn compute_coverage(
    spec_name: &str,
    gates: &GatesSpec,
    feature: Option<&FeatureSpec>,
) -> CoverageReport {
    // Collect declared requirement IDs from the feature spec.
    let req_ids: BTreeSet<String> = feature
        .map(|f| f.requirements.iter().map(|r| r.id.clone()).collect())
        .unwrap_or_default();

    // Collect all criterion requirement references, deduplicated.
    let criterion_refs: BTreeSet<String> = gates
        .criteria
        .iter()
        .flat_map(|c| c.requirements.iter().cloned())
        .collect();

    // Set operations.
    let covered: Vec<String> = req_ids.intersection(&criterion_refs).cloned().collect();
    let uncovered: Vec<String> = req_ids.difference(&criterion_refs).cloned().collect();
    let orphaned: Vec<String> = criterion_refs.difference(&req_ids).cloned().collect();

    let total_requirements = req_ids.len();
    let coverage_pct = if total_requirements == 0 {
        100.0
    } else {
        covered.len() as f64 / total_requirements as f64 * 100.0
    };

    CoverageReport {
        spec: spec_name.to_string(),
        total_requirements,
        covered,
        uncovered,
        orphaned,
        coverage_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::Criterion;
    use assay_types::feature_spec::{Obligation, Priority, Requirement};

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
            when: None,
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
            auto_promote: false,
        }
    }

    #[test]
    fn zero_requirements_returns_100_percent() {
        let gates = make_gates(vec![]);
        let report = compute_coverage("test", &gates, None);

        assert_eq!(report.total_requirements, 0);
        assert_eq!(report.coverage_pct, 100.0);
        assert!(report.covered.is_empty());
        assert!(report.uncovered.is_empty());
        assert!(report.orphaned.is_empty());
    }

    #[test]
    fn full_coverage() {
        let feature = make_feature(vec![
            make_requirement("REQ-AUTH-001"),
            make_requirement("REQ-AUTH-002"),
        ]);
        let gates = make_gates(vec![
            make_criterion("c1", &["REQ-AUTH-001"]),
            make_criterion("c2", &["REQ-AUTH-002"]),
        ]);

        let report = compute_coverage("auth-flow", &gates, Some(&feature));

        assert_eq!(report.total_requirements, 2);
        assert_eq!(report.coverage_pct, 100.0);
        assert_eq!(report.covered, vec!["REQ-AUTH-001", "REQ-AUTH-002"]);
        assert!(report.uncovered.is_empty());
        assert!(report.orphaned.is_empty());
    }

    #[test]
    fn partial_coverage() {
        let feature = make_feature(vec![
            make_requirement("REQ-AUTH-001"),
            make_requirement("REQ-AUTH-002"),
            make_requirement("REQ-AUTH-003"),
        ]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-AUTH-001"])]);

        let report = compute_coverage("auth-flow", &gates, Some(&feature));

        assert_eq!(report.total_requirements, 3);
        assert!((report.coverage_pct - 100.0 / 3.0).abs() < 1e-10);
        assert_eq!(report.covered, vec!["REQ-AUTH-001"]);
        assert_eq!(report.uncovered, vec!["REQ-AUTH-002", "REQ-AUTH-003"]);
        assert!(report.orphaned.is_empty());
    }

    #[test]
    fn orphaned_only() {
        let feature = make_feature(vec![make_requirement("REQ-AUTH-001")]);
        let gates = make_gates(vec![make_criterion(
            "c1",
            &["REQ-AUTH-001", "REQ-UNKNOWN-999"],
        )]);

        let report = compute_coverage("auth-flow", &gates, Some(&feature));

        assert_eq!(report.total_requirements, 1);
        assert_eq!(report.coverage_pct, 100.0);
        assert_eq!(report.covered, vec!["REQ-AUTH-001"]);
        assert!(report.uncovered.is_empty());
        assert_eq!(report.orphaned, vec!["REQ-UNKNOWN-999"]);
    }

    #[test]
    fn both_uncovered_and_orphaned() {
        let feature = make_feature(vec![
            make_requirement("REQ-AUTH-001"),
            make_requirement("REQ-AUTH-002"),
        ]);
        let gates = make_gates(vec![make_criterion(
            "c1",
            &["REQ-AUTH-001", "REQ-ORPHAN-001"],
        )]);

        let report = compute_coverage("auth-flow", &gates, Some(&feature));

        assert_eq!(report.total_requirements, 2);
        assert_eq!(report.coverage_pct, 50.0);
        assert_eq!(report.covered, vec!["REQ-AUTH-001"]);
        assert_eq!(report.uncovered, vec!["REQ-AUTH-002"]);
        assert_eq!(report.orphaned, vec!["REQ-ORPHAN-001"]);
    }

    #[test]
    fn feature_none_with_criterion_refs_shows_orphaned() {
        let gates = make_gates(vec![make_criterion("c1", &["REQ-GHOST-001"])]);

        let report = compute_coverage("test", &gates, None);

        assert_eq!(report.total_requirements, 0);
        assert_eq!(report.coverage_pct, 100.0);
        assert!(report.covered.is_empty());
        assert!(report.uncovered.is_empty());
        assert_eq!(report.orphaned, vec!["REQ-GHOST-001"]);
    }

    #[test]
    fn vecs_are_sorted() {
        let feature = make_feature(vec![
            make_requirement("REQ-Z-001"),
            make_requirement("REQ-A-001"),
            make_requirement("REQ-M-001"),
        ]);
        let gates = make_gates(vec![make_criterion("c1", &["REQ-M-001"])]);

        let report = compute_coverage("test", &gates, Some(&feature));

        // BTreeSet produces sorted output.
        assert_eq!(report.covered, vec!["REQ-M-001"]);
        assert_eq!(report.uncovered, vec!["REQ-A-001", "REQ-Z-001"]);
    }
}
