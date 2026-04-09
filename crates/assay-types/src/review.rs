//! Spec review report types.
//!
//! A [`ReviewReport`] captures the results of running structural and
//! optional agent-quality checks against a spec. Each individual check
//! is represented as a [`ReviewCheck`] with a [`ReviewCheckKind`].

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The kind of review check: structural (machine-checkable) or
/// agent quality (LLM-evaluated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewCheckKind {
    /// Machine-checkable structural completeness check.
    Structural,
    /// LLM-evaluated quality assessment (via evaluator subprocess).
    AgentQuality,
}

/// A single review check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewCheck {
    /// Check identifier, e.g. "req-coverage", "acceptance-criteria".
    pub name: String,

    /// Whether this is a structural or agent-quality check.
    pub kind: ReviewCheckKind,

    /// Whether the check was skipped (not applicable for this spec type).
    /// When true, `passed` is also true and the result should not count
    /// against the failure tally.
    #[serde(default)]
    pub skipped: bool,

    /// Whether the check passed (always true when `skipped` is true).
    pub passed: bool,

    /// Human-readable summary of the check result.
    pub message: String,

    /// Optional detailed evidence (e.g. list of uncovered REQ-IDs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// A complete review report for a spec.
///
/// Contains the results of all checks (structural and optionally agent-quality),
/// along with summary counts and metadata for persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewReport {
    /// Spec slug this review was generated for.
    pub spec: String,

    /// Unique run identifier (timestamp-based, set by `save_review()`).
    /// `None` for unsaved in-memory reports; always `Some` after persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,

    /// When the review was executed.
    pub timestamp: DateTime<Utc>,

    /// All check results in execution order.
    pub checks: Vec<ReviewCheck>,

    /// Number of checks that passed.
    pub passed: usize,

    /// Number of checks that failed.
    pub failed: usize,

    /// Number of checks that were skipped (not applicable).
    pub skipped: usize,
}

/// Summary of a single failed gate criterion for diagnostic persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FailedCriterionSummary {
    /// Name of the criterion that failed.
    pub criterion_name: String,
    /// The command that was executed (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Exit code from the command (if applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Stderr snippet from the failed command (truncated).
    #[serde(default)]
    pub stderr_snippet: String,
}

/// Persistent record of gate failures from a pipeline run.
///
/// Written to `.assay/reviews/<spec>/<run-id>-gates.json` when a run
/// completes with gate failures. Read by `assay spec review` to show
/// failure details alongside structural checks.
///
/// # Security note
///
/// Diagnostic files may contain output captured from gate commands (see
/// [`FailedCriterionSummary::stderr_snippet`]). Gate commands are arbitrary
/// shell commands, so their output could theoretically include sensitive data
/// (API keys, tokens, passwords) if a failing command printed them to stderr.
/// Treat `.assay/reviews/` with the same access controls as `.assay/sessions/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateDiagnostic {
    /// Spec slug this diagnostic is for.
    pub spec: String,
    /// Run identifier (matches the session's run_id).
    pub run_id: String,
    /// When the gate evaluation completed.
    pub timestamp: DateTime<Utc>,
    /// Details of each failed criterion.
    pub failed_criteria: Vec<FailedCriterionSummary>,
    /// Number of criteria that passed.
    pub passed: usize,
    /// Number of criteria that failed.
    pub failed: usize,
    /// Index of the checkpoint that failed (0-based). `None` for session-end gates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_index: Option<u32>,
    /// The pipeline phase at which the gate was evaluated.
    /// Defaults to `SessionEnd` for pre-M024 diagnostics (D028).
    #[serde(default)]
    pub session_phase: CheckpointPhase,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "review-check-kind",
        generate: || schemars::schema_for!(ReviewCheckKind),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "review-check",
        generate: || schemars::schema_for!(ReviewCheck),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "review-report",
        generate: || schemars::schema_for!(ReviewReport),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "failed-criterion-summary",
        generate: || schemars::schema_for!(FailedCriterionSummary),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-diagnostic",
        generate: || schemars::schema_for!(GateDiagnostic),
    }
}

/// The phase in a pipeline session at which a checkpoint gate was evaluated.
///
/// Lives in `review` (not `gate` or `pipeline`) per D029 — phase is a
/// diagnostic concern that belongs next to [`GateDiagnostic`].
///
/// Accessed as `assay_types::review::CheckpointPhase` — deliberately NOT
/// re-exported at the crate root to avoid collision with the workflow
/// `SessionPhase` in `work_session`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckpointPhase {
    /// Gate was evaluated at a tool-call checkpoint (`n` = tool call count).
    AtToolCall {
        /// Number of tool calls emitted when the checkpoint fired.
        n: u32,
    },
    /// Gate was evaluated when a specific event type was observed.
    ///
    /// Deserializes from both `on_event` (canonical) and `at_event` (legacy alias)
    /// JSON tags for backward compatibility with persisted [`GateDiagnostic`] JSON.
    #[serde(alias = "at_event")]
    OnEvent {
        /// The event type tag that triggered the checkpoint.
        event_type: String,
    },
    /// Gate was evaluated at the end of the session.
    #[default]
    SessionEnd,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "checkpoint-phase",
        generate: || schemars::schema_for!(CheckpointPhase),
    }
}

// Legacy alias so consumers looking up the old registry key still find it.
inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "checkpoint-session-phase",
        generate: || schemars::schema_for!(CheckpointPhase),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_phase_on_event_alias_roundtrip() {
        // Legacy JSON uses `at_event` tag; must deserialize to OnEvent variant.
        let json = r#"{"type":"at_event","event_type":"foo"}"#;
        let phase: CheckpointPhase =
            serde_json::from_str(json).expect("deserialize at_event alias");
        assert_eq!(
            phase,
            CheckpointPhase::OnEvent {
                event_type: "foo".to_string()
            }
        );

        // Canonical JSON uses `on_event` tag.
        let canonical = r#"{"type":"on_event","event_type":"foo"}"#;
        let phase2: CheckpointPhase =
            serde_json::from_str(canonical).expect("deserialize on_event");
        assert_eq!(
            phase2,
            CheckpointPhase::OnEvent {
                event_type: "foo".to_string()
            }
        );
    }
}
