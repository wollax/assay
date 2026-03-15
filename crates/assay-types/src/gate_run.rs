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
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-run-record",
        generate: || schemars::schema_for!(GateRunRecord),
    }
}
