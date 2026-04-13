//! Gate run summary types for evaluation results.
//!
//! These types represent the aggregate output of evaluating all criteria
//! in a spec. They are defined in `assay-types` (not `assay-core`) because
//! downstream consumers (MCP server, CLI, future persistence layer) need
//! to deserialize and schema-validate them independently.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

use crate::GateResult;
use crate::enforcement::{Enforcement, EnforcementSummary};
use crate::precondition::PreconditionStatus;
use crate::resolved_gate::CriterionSource;

/// Summary of evaluating all criteria in a spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GateRunSummary {
    /// Spec name that was evaluated.
    pub spec_name: String,
    /// Results for each criterion that was evaluated or skipped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<CriterionResult>,
    /// Number of criteria that passed.
    pub passed: usize,
    /// Number of criteria that failed.
    pub failed: usize,
    /// Number of criteria skipped (descriptive-only, no cmd).
    pub skipped: usize,
    /// Total wall-clock duration for all evaluations in milliseconds.
    pub total_duration_ms: u64,
    /// Enforcement-level breakdown of results (excludes skipped criteria).
    #[serde(default)]
    pub enforcement: EnforcementSummary,
}

/// A criterion paired with its evaluation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CriterionResult {
    /// The name of the criterion that was evaluated.
    pub criterion_name: String,
    /// The gate result, or `None` if skipped (no cmd).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<GateResult>,
    /// Resolved enforcement level for this criterion (Required or Advisory).
    #[serde(default)]
    pub enforcement: Enforcement,
    /// Where this criterion originated (own, parent, or library). Omitted when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CriterionSource>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-run-summary",
        generate: || schemars::schema_for!(GateRunSummary),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criterion-result",
        generate: || schemars::schema_for!(CriterionResult),
    }
}

/// Truncation metadata when the diff was truncated to fit the evaluator's token budget.
///
/// Present only when truncation occurred (diff exceeded token budget).
/// Omitted entirely when the diff fit within budget (clean passthrough).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct DiffTruncation {
    /// Byte size of the original diff before truncation.
    pub original_bytes: u64,
    /// Byte size of the diff after truncation.
    pub truncated_bytes: u64,
    /// Files included in the truncated diff.
    pub included_files: Vec<String>,
    /// Files omitted from the truncated diff (present in original, absent after truncation).
    pub omitted_files: Vec<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "diff-truncation",
        generate: || schemars::schema_for!(DiffTruncation),
    }
}

/// A complete, versioned record of a single gate evaluation run.
///
/// Wraps [`GateRunSummary`] with metadata for persistence and audit.
/// Uses `deny_unknown_fields` — records are versioned artifacts;
/// field mismatches should fail loudly. `assay_version` supports
/// future schema migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateRunRecord {
    /// Unique run identifier: `<timestamp>-<6-char-hex>` (e.g., `20260304T223015Z-a3f1b2`).
    pub run_id: String,
    /// Version of assay that produced this record (from `env!("CARGO_PKG_VERSION")`).
    pub assay_version: String,
    /// UTC timestamp when the evaluation started.
    pub timestamp: DateTime<Utc>,
    /// Working directory used for evaluation, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    /// The complete gate run summary with all criterion results.
    pub summary: GateRunSummary,
    /// Truncation metadata for the diff passed to the evaluator.
    /// Present only when truncation occurred; omitted when diff fit within budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_truncation: Option<DiffTruncation>,
    /// Whether this run was blocked by a precondition failure. When `Some(true)`,
    /// the gate was never evaluated — the summary contains zeroed counters.
    /// Absent (`None`) for normal evaluation runs. Backward-compatible: old
    /// records without this field deserialize to `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precondition_blocked: Option<bool>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-run-record",
        generate: || schemars::schema_for!(GateRunRecord),
    }
}

/// Outcome of a gate evaluation, distinguishing normal evaluation from
/// precondition failures.
///
/// `GateEvalOutcome` is an in-memory return type only. Callers that persist
/// results save the inner `GateRunSummary` from `Evaluated` via
/// [`crate::GateRunRecord`]. `PreconditionFailed` results are NOT stored in
/// run history — they block evaluation before any criteria run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum GateEvalOutcome {
    /// All preconditions passed; the inner value is the full gate run summary.
    Evaluated(GateRunSummary),
    /// One or more preconditions failed; evaluation was not attempted.
    PreconditionFailed(PreconditionStatus),
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-eval-outcome",
        generate: || schemars::schema_for!(GateEvalOutcome),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enforcement::EnforcementSummary;
    use crate::precondition::{CommandStatus, PreconditionStatus, RequireStatus};
    use crate::resolved_gate::CriterionSource;

    fn make_empty_summary(spec_name: &str) -> GateRunSummary {
        GateRunSummary {
            spec_name: spec_name.to_string(),
            results: vec![],
            passed: 0,
            failed: 0,
            skipped: 0,
            total_duration_ms: 0,
            enforcement: EnforcementSummary::default(),
        }
    }

    fn make_precondition_status() -> PreconditionStatus {
        PreconditionStatus {
            requires: vec![RequireStatus {
                spec_slug: "auth-flow".to_string(),
                passed: false,
            }],
            commands: vec![CommandStatus {
                command: "docker ps".to_string(),
                passed: true,
                output: None,
            }],
        }
    }

    // Test 1: GateEvalOutcome::Evaluated(summary) roundtrips JSON
    #[test]
    fn gate_eval_outcome_evaluated_roundtrip() {
        let summary = make_empty_summary("my-spec");
        let outcome = GateEvalOutcome::Evaluated(summary.clone());
        let json = serde_json::to_string(&outcome).expect("serialize Evaluated");
        let back: GateEvalOutcome = serde_json::from_str(&json).expect("deserialize Evaluated");
        assert_eq!(back, outcome);
    }

    // Test 2: GateEvalOutcome::PreconditionFailed(status) roundtrips JSON
    #[test]
    fn gate_eval_outcome_precondition_failed_roundtrip() {
        let status = make_precondition_status();
        let outcome = GateEvalOutcome::PreconditionFailed(status.clone());
        let json = serde_json::to_string(&outcome).expect("serialize PreconditionFailed");
        let back: GateEvalOutcome =
            serde_json::from_str(&json).expect("deserialize PreconditionFailed");
        assert_eq!(back, outcome);
    }

    // Test 3: Evaluated and PreconditionFailed are distinguishable via "outcome" tag
    #[test]
    fn gate_eval_outcome_tag_distinguishable() {
        let evaluated = GateEvalOutcome::Evaluated(make_empty_summary("spec-a"));
        let failed = GateEvalOutcome::PreconditionFailed(make_precondition_status());

        let eval_json = serde_json::to_string(&evaluated).expect("serialize Evaluated");
        let fail_json = serde_json::to_string(&failed).expect("serialize PreconditionFailed");

        let eval_val: serde_json::Value = serde_json::from_str(&eval_json).unwrap();
        let fail_val: serde_json::Value = serde_json::from_str(&fail_json).unwrap();

        assert_eq!(eval_val["outcome"], "evaluated");
        assert_eq!(fail_val["outcome"], "precondition_failed");
    }

    // Test 4: CriterionResult with source: Some(CriterionSource::Own) roundtrips JSON
    #[test]
    fn criterion_result_source_own_roundtrip() {
        let cr = CriterionResult {
            criterion_name: "cargo-test".to_string(),
            result: None,
            enforcement: Enforcement::Required,
            source: Some(CriterionSource::Own),
        };
        let json = serde_json::to_string(&cr).expect("serialize with source");
        let back: CriterionResult = serde_json::from_str(&json).expect("deserialize with source");
        assert_eq!(back, cr);
        assert_eq!(back.source, Some(CriterionSource::Own));
    }

    // Test 5: CriterionResult with source: None roundtrips JSON and omits "source" key
    #[test]
    fn criterion_result_source_none_omitted() {
        let cr = CriterionResult {
            criterion_name: "cargo-test".to_string(),
            result: None,
            enforcement: Enforcement::Required,
            source: None,
        };
        let json = serde_json::to_string(&cr).expect("serialize with source None");
        assert!(
            !json.contains("source"),
            "source field should be omitted when None, got: {json}"
        );
        let back: CriterionResult =
            serde_json::from_str(&json).expect("deserialize with source None");
        assert_eq!(back, cr);
        assert_eq!(back.source, None);
    }

    // Test 6: Old CriterionResult JSON (no source field) deserializes into source: None
    #[test]
    fn criterion_result_backward_compat_no_source_field() {
        // JSON that would have been produced before source field existed
        let old_json = r#"{"criterion_name":"cargo-test","enforcement":"required"}"#;
        let cr: CriterionResult = serde_json::from_str(old_json).expect("deserialize old JSON");
        assert_eq!(cr.criterion_name, "cargo-test");
        assert_eq!(
            cr.source, None,
            "old JSON without source should produce source: None"
        );
    }

    // --- GateRunRecord::precondition_blocked tests ---

    // Test: GateRunRecord with precondition_blocked: Some(true) roundtrips JSON
    #[test]
    fn gate_run_record_precondition_blocked_some_true_roundtrip() {
        use chrono::TimeZone;
        let ts = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let record = GateRunRecord {
            run_id: "20231114T221320Z-a1b2c3".to_string(),
            assay_version: "0.7.0-test".to_string(),
            timestamp: ts,
            working_dir: None,
            summary: make_empty_summary("prec-blocked-spec"),
            diff_truncation: None,
            precondition_blocked: Some(true),
        };
        let json = serde_json::to_string(&record).expect("serialize with precondition_blocked");
        let back: GateRunRecord =
            serde_json::from_str(&json).expect("deserialize with precondition_blocked");
        assert_eq!(back.precondition_blocked, Some(true));
        assert_eq!(back.run_id, record.run_id);
    }

    // Test: GateRunRecord without precondition_blocked field (old JSON) deserializes to None
    #[test]
    fn gate_run_record_precondition_blocked_backward_compat() {
        // Old record JSON that doesn't have precondition_blocked field
        let old_json = r#"{
            "run_id": "20231114T221320Z-a1b2c3",
            "assay_version": "0.6.0",
            "timestamp": "2023-11-14T22:13:20Z",
            "summary": {
                "spec_name": "old-spec",
                "passed": 1,
                "failed": 0,
                "skipped": 0,
                "total_duration_ms": 100
            }
        }"#;
        let record: GateRunRecord = serde_json::from_str(old_json)
            .expect("deserialize old record without precondition_blocked");
        assert_eq!(
            record.precondition_blocked, None,
            "old record without precondition_blocked should deserialize to None"
        );
    }

    // Test: GateRunRecord with precondition_blocked: None omits the field from serialized JSON
    #[test]
    fn gate_run_record_precondition_blocked_none_omitted() {
        use chrono::TimeZone;
        let ts = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let record = GateRunRecord {
            run_id: "20231114T221320Z-a1b2c3".to_string(),
            assay_version: "0.7.0-test".to_string(),
            timestamp: ts,
            working_dir: None,
            summary: make_empty_summary("prec-none-spec"),
            diff_truncation: None,
            precondition_blocked: None,
        };
        let json =
            serde_json::to_string(&record).expect("serialize with precondition_blocked None");
        assert!(
            !json.contains("precondition_blocked"),
            "precondition_blocked should be omitted when None, got: {json}"
        );
    }

    // --- end GateRunRecord::precondition_blocked tests ---

    // Test 7: GateEvalOutcome schema registered via inventory (name "gate-eval-outcome")
    #[test]
    fn gate_eval_outcome_schema_registered() {
        use crate::schema_registry;
        let names: Vec<&str> = schema_registry::all_entries().map(|e| e.name).collect();
        assert!(
            names.contains(&"gate-eval-outcome"),
            "gate-eval-outcome should be registered, found: {names:?}"
        );
    }
}
