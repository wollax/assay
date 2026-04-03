//! Requirements coverage report types.
//!
//! A [`CoverageReport`] summarizes which feature-spec requirements are
//! covered by gate criteria, which are uncovered, and which criterion
//! `requirements` references point to non-existent requirement IDs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Report produced by `compute_coverage()` summarizing requirement coverage.
///
/// - **covered**: REQ-IDs that have at least one criterion referencing them.
/// - **uncovered**: REQ-IDs that no criterion references.
/// - **orphaned**: criterion `requirements` entries that don't match any declared REQ-ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CoverageReport {
    /// Spec slug this report was generated for.
    pub spec: String,

    /// Total number of declared requirements in the feature spec.
    pub total_requirements: usize,

    /// REQ-IDs that have at least one criterion referencing them.
    pub covered: Vec<String>,

    /// REQ-IDs that no criterion references.
    pub uncovered: Vec<String>,

    /// Criterion `requirements` entries that don't match any declared REQ-ID.
    pub orphaned: Vec<String>,

    /// Coverage percentage: `covered.len() / total_requirements * 100`.
    /// Returns `100.0` when `total_requirements == 0` (nothing to miss).
    pub coverage_pct: f64,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "coverage-report",
        generate: || schemars::schema_for!(CoverageReport),
    }
}
