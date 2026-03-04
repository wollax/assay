//! Enforcement level types for gate criteria.
//!
//! Determines whether a criterion failure blocks the gate (required)
//! or is informational only (advisory).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Enforcement level for a gate criterion.
///
/// Determines whether a criterion failure blocks the gate (required)
/// or is informational only (advisory).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    /// Failure blocks the gate. This is the default.
    #[default]
    Required,
    /// Failure is informational; does not block the gate.
    Advisory,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "enforcement",
        generate: || schemars::schema_for!(Enforcement),
    }
}

/// Gate-level configuration section.
///
/// Parsed from `[gate]` in spec TOML files. Provides spec-wide defaults
/// that individual criteria can override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateSection {
    /// Default enforcement level for all criteria in this spec.
    #[serde(default)]
    pub enforcement: Enforcement,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-section",
        generate: || schemars::schema_for!(GateSection),
    }
}

/// Enforcement breakdown in a gate run summary.
///
/// Always present on `GateRunSummary`, with counts defaulting to 0.
/// Only counts executable criteria (skipped criteria are excluded).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EnforcementSummary {
    pub required_passed: usize,
    pub required_failed: usize,
    pub advisory_passed: usize,
    pub advisory_failed: usize,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "enforcement-summary",
        generate: || schemars::schema_for!(EnforcementSummary),
    }
}
