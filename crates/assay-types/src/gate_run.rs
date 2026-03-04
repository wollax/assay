//! Gate run summary types for evaluation results.
//!
//! These types represent the aggregate output of evaluating all criteria
//! in a spec. They are defined in `assay-types` (not `assay-core`) because
//! downstream consumers (MCP server, CLI, future persistence layer) need
//! to deserialize and schema-validate them independently.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::GateResult;

/// Summary of evaluating all criteria in a spec.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
}

/// A criterion paired with its evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CriterionResult {
    /// The name of the criterion that was evaluated.
    pub criterion_name: String,
    /// The gate result, or `None` if skipped (no cmd).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<GateResult>,
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
