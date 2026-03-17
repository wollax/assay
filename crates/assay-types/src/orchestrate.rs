//! Orchestrator state types for multi-session parallel execution.
//!
//! These types define the data contract for orchestrator state persistence,
//! consumed by the executor (S02), merge runner (S03), MCP tools (S06),
//! and CLI status commands. All types are serializable and schema-generating
//! for structured observability.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema_registry;

/// Run state of a single session in the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionRunState {
    /// Session is queued and waiting for dependencies/concurrency slot.
    Pending,
    /// Session is currently executing.
    Running,
    /// Session completed successfully.
    Completed,
    /// Session failed during execution.
    Failed,
    /// Session was skipped (e.g., upstream dependency failed).
    Skipped,
}

/// Failure policy for orchestrated runs.
///
/// Defaults to [`FailurePolicy::SkipDependents`], which allows independent
/// sessions to continue while skipping only those that depend on the failed session.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
    /// Skip dependents of failed sessions, continue independent sessions.
    #[default]
    SkipDependents,
    /// Abort the entire run on first failure — no new sessions are dispatched.
    Abort,
}

/// High-level phase of an orchestrated run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratorPhase {
    /// Sessions are still being dispatched/executed.
    Running,
    /// All sessions completed successfully.
    Completed,
    /// At least one session failed; others may have completed or been skipped.
    PartialFailure,
    /// Run was aborted (due to [`FailurePolicy::Abort`] or external signal).
    Aborted,
}

/// Per-session status snapshot for observability.
///
/// Persisted as part of [`OrchestratorStatus`] in
/// `.assay/orchestrator/<run_id>/state.json` and readable by the
/// `orchestrate_status` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionStatus {
    /// Effective session name (from manifest or spec name fallback).
    pub name: String,
    /// Spec path or name this session evaluates.
    pub spec: String,
    /// Current run state.
    pub state: SessionRunState,
    /// When the session started executing (None if still pending/skipped).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// When the session finished (None if still pending/running).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Wall-clock duration in seconds (None if not yet completed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    /// Error message if the session failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Reason the session was skipped (e.g., "upstream 'auth' failed").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

/// Orchestrator-level status snapshot, persisted for status queries.
///
/// Written to `.assay/orchestrator/<run_id>/state.json` after each session
/// completion. Designed to be read by the `orchestrate_status` MCP tool
/// and CLI status commands.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrchestratorStatus {
    /// Unique identifier for this orchestrated run.
    pub run_id: String,
    /// Current phase of the run.
    pub phase: OrchestratorPhase,
    /// Failure policy in effect for this run.
    pub failure_policy: FailurePolicy,
    /// Per-session status snapshots.
    pub sessions: Vec<SessionStatus>,
    /// When the orchestrated run started.
    pub started_at: DateTime<Utc>,
    /// When the run completed (None if still running).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

// ── Schema registry submissions ──────────────────────────────────────

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "session-run-state",
        generate: || schemars::schema_for!(SessionRunState),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "failure-policy",
        generate: || schemars::schema_for!(FailurePolicy),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "orchestrator-phase",
        generate: || schemars::schema_for!(OrchestratorPhase),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "session-status",
        generate: || schemars::schema_for!(SessionStatus),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "orchestrator-status",
        generate: || schemars::schema_for!(OrchestratorStatus),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_run_state_serde_roundtrip() {
        let states = vec![
            SessionRunState::Pending,
            SessionRunState::Running,
            SessionRunState::Completed,
            SessionRunState::Failed,
            SessionRunState::Skipped,
        ];
        for state in &states {
            let json = serde_json::to_string(state).unwrap();
            let back: SessionRunState = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, state);
        }
        // Verify snake_case serialization
        assert_eq!(
            serde_json::to_string(&SessionRunState::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&SessionRunState::Skipped).unwrap(),
            "\"skipped\""
        );
    }

    #[test]
    fn failure_policy_serde_roundtrip() {
        let policies = vec![FailurePolicy::SkipDependents, FailurePolicy::Abort];
        for policy in &policies {
            let json = serde_json::to_string(policy).unwrap();
            let back: FailurePolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, policy);
        }
        assert_eq!(
            serde_json::to_string(&FailurePolicy::SkipDependents).unwrap(),
            "\"skip_dependents\""
        );
        assert_eq!(
            serde_json::to_string(&FailurePolicy::Abort).unwrap(),
            "\"abort\""
        );
    }

    #[test]
    fn failure_policy_default_is_skip_dependents() {
        assert_eq!(FailurePolicy::default(), FailurePolicy::SkipDependents);
    }

    #[test]
    fn orchestrator_phase_serde_roundtrip() {
        let phases = vec![
            OrchestratorPhase::Running,
            OrchestratorPhase::Completed,
            OrchestratorPhase::PartialFailure,
            OrchestratorPhase::Aborted,
        ];
        for phase in &phases {
            let json = serde_json::to_string(phase).unwrap();
            let back: OrchestratorPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, phase);
        }
        assert_eq!(
            serde_json::to_string(&OrchestratorPhase::PartialFailure).unwrap(),
            "\"partial_failure\""
        );
    }

    #[test]
    fn session_status_serde_roundtrip() {
        let status = SessionStatus {
            name: "auth-flow".to_string(),
            spec: "specs/auth.toml".to_string(),
            state: SessionRunState::Failed,
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            duration_secs: Some(42.5),
            error: Some("gate evaluation failed".to_string()),
            skip_reason: None,
        };
        let json = serde_json::to_string_pretty(&status).unwrap();
        let back: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "auth-flow");
        assert_eq!(back.state, SessionRunState::Failed);
        assert!(back.error.is_some());
        assert!(back.skip_reason.is_none());
    }

    #[test]
    fn session_status_skipped_roundtrip() {
        let status = SessionStatus {
            name: "payment".to_string(),
            spec: "specs/payment.toml".to_string(),
            state: SessionRunState::Skipped,
            started_at: None,
            completed_at: None,
            duration_secs: None,
            error: None,
            skip_reason: Some("upstream 'auth-flow' failed".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.state, SessionRunState::Skipped);
        assert_eq!(
            back.skip_reason.as_deref(),
            Some("upstream 'auth-flow' failed")
        );
    }

    #[test]
    fn orchestrator_status_full_roundtrip() {
        let now = Utc::now();
        let status = OrchestratorStatus {
            run_id: "01JTEST123".to_string(),
            phase: OrchestratorPhase::PartialFailure,
            failure_policy: FailurePolicy::SkipDependents,
            sessions: vec![
                SessionStatus {
                    name: "auth".to_string(),
                    spec: "specs/auth.toml".to_string(),
                    state: SessionRunState::Completed,
                    started_at: Some(now),
                    completed_at: Some(now),
                    duration_secs: Some(30.0),
                    error: None,
                    skip_reason: None,
                },
                SessionStatus {
                    name: "payment".to_string(),
                    spec: "specs/payment.toml".to_string(),
                    state: SessionRunState::Failed,
                    started_at: Some(now),
                    completed_at: Some(now),
                    duration_secs: Some(15.5),
                    error: Some("agent timeout".to_string()),
                    skip_reason: None,
                },
                SessionStatus {
                    name: "checkout".to_string(),
                    spec: "specs/checkout.toml".to_string(),
                    state: SessionRunState::Skipped,
                    started_at: None,
                    completed_at: None,
                    duration_secs: None,
                    error: None,
                    skip_reason: Some("upstream 'payment' failed".to_string()),
                },
            ],
            started_at: now,
            completed_at: Some(now),
        };
        let json = serde_json::to_string_pretty(&status).unwrap();
        let back: OrchestratorStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.run_id, "01JTEST123");
        assert_eq!(back.phase, OrchestratorPhase::PartialFailure);
        assert_eq!(back.failure_policy, FailurePolicy::SkipDependents);
        assert_eq!(back.sessions.len(), 3);
    }

    #[test]
    fn session_status_deny_unknown_fields() {
        let json = r#"{"name":"x","spec":"s","state":"pending","unknown_field":true}"#;
        let result = serde_json::from_str::<SessionStatus>(json);
        assert!(result.is_err(), "should reject unknown fields");
    }

    #[test]
    fn orchestrator_status_deny_unknown_fields() {
        let json = r#"{"run_id":"x","phase":"running","failure_policy":"abort","sessions":[],"started_at":"2026-01-01T00:00:00Z","extra":1}"#;
        let result = serde_json::from_str::<OrchestratorStatus>(json);
        assert!(result.is_err(), "should reject unknown fields");
    }
}
