//! Formatted gate evidence for PR body consumption.
//!
//! [`FormattedEvidence`] is the output of formatting a gate run record
//! into markdown. It carries both the truncated PR body (fit for GitHub's
//! 65,536-byte limit) and the full untruncated report for disk persistence.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The result of formatting gate evidence for PR consumption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FormattedEvidence {
    /// Markdown formatted for the PR body, truncated to fit the character limit.
    pub pr_body: String,
    /// Full untruncated markdown report for disk persistence.
    pub full_report: String,
    /// Whether the PR body was truncated to fit the character limit.
    pub truncated: bool,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "formatted-evidence",
        generate: || schemars::schema_for!(FormattedEvidence),
    }
}
