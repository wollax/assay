//! Structured validation diagnostics for spec files.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Severity level of a validation diagnostic.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Blocks validity — spec cannot be used.
    Error,
    /// Advisory — spec is usable but has issues worth addressing.
    Warning,
    /// Informational — suggestion or note.
    Info,
}

/// A single validation diagnostic with location, severity, and message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: Severity,
    /// Location reference (e.g., "name", "criteria[2].cmd", "depends").
    pub location: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Result of validating one or more specs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ValidationResult {
    /// The spec name/slug that was validated.
    pub spec: String,
    /// Whether the spec is valid (no error-severity diagnostics).
    pub valid: bool,
    /// All diagnostics found.
    pub diagnostics: Vec<Diagnostic>,
    /// Summary counts by severity.
    pub summary: DiagnosticSummary,
}

/// Counts of diagnostics by severity level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DiagnosticSummary {
    /// Number of error-level diagnostics.
    pub errors: usize,
    /// Number of warning-level diagnostics.
    pub warnings: usize,
    /// Number of info-level diagnostics.
    pub info: usize,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "severity",
        generate: || schemars::schema_for!(Severity),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "diagnostic",
        generate: || schemars::schema_for!(Diagnostic),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "validation-result",
        generate: || schemars::schema_for!(ValidationResult),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "diagnostic-summary",
        generate: || schemars::schema_for!(DiagnosticSummary),
    }
}
