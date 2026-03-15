//! Work session types for on-disk session persistence.
//!
//! A [`WorkSession`] tracks the lifecycle of a single spec evaluation run,
//! linking worktrees, agent invocations, and gate runs through a linear
//! state machine ([`SessionPhase`]).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Phase of a work session lifecycle.
///
/// Sessions follow a linear pipeline: Created → AgentRunning → GateEvaluated → Completed.
/// Any phase can transition to Abandoned (the escape hatch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    /// Session created, not yet started.
    Created,
    /// Agent is actively running.
    AgentRunning,
    /// Gate evaluation has completed.
    GateEvaluated,
    /// Session completed successfully.
    Completed,
    /// Session was abandoned.
    Abandoned,
}

impl SessionPhase {
    /// Returns whether transitioning from this phase to `next` is valid.
    ///
    /// Valid transitions follow the linear pipeline:
    /// - Created → AgentRunning
    /// - AgentRunning → GateEvaluated
    /// - GateEvaluated → Completed
    /// - Any non-terminal → Abandoned
    ///
    /// Terminal phases (Completed, Abandoned) cannot transition to anything.
    pub fn can_transition_to(&self, next: SessionPhase) -> bool {
        if self.is_terminal() {
            return false;
        }

        matches!(
            (self, next),
            (Self::Created, Self::AgentRunning)
                | (Self::AgentRunning, Self::GateEvaluated)
                | (Self::GateEvaluated, Self::Completed)
                | (_, Self::Abandoned)
        )
    }

    /// Returns whether this phase is terminal (no further transitions allowed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Abandoned)
    }
}

/// A recorded phase transition within a work session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PhaseTransition {
    /// The phase transitioned from.
    pub from: SessionPhase,
    /// The phase transitioned to.
    pub to: SessionPhase,
    /// When the transition occurred.
    pub timestamp: DateTime<Utc>,
    /// What triggered the transition (e.g., "agent_started", "gate_passed").
    pub trigger: String,
    /// Optional notes about the transition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "phase-transition",
        generate: || schemars::schema_for!(PhaseTransition),
    }
}

/// Details about the agent invocation for this session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AgentInvocation {
    /// Name of the spec being worked on.
    pub spec_name: String,
    /// The command used to invoke the agent.
    pub command: String,
    /// The model used by the agent, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "agent-invocation",
        generate: || schemars::schema_for!(AgentInvocation),
    }
}

/// An on-disk work session linking worktrees, agent invocations, and gate runs.
///
/// This is distinct from [`AgentSession`](crate::session::AgentSession), which is
/// an in-memory crash-recoverable session for gate evaluation. `WorkSession` is the
/// persistent record of an entire spec evaluation lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkSession {
    /// Unique session identifier (ULID stored as string).
    pub id: String,
    /// Name of the spec this session is for.
    pub spec_name: String,
    /// Path to the worktree associated with this session.
    pub worktree_path: PathBuf,
    /// Current phase of the session lifecycle.
    pub phase: SessionPhase,
    /// History of phase transitions.
    pub transitions: Vec<PhaseTransition>,
    /// Details about the agent invocation.
    pub agent: AgentInvocation,
    /// IDs of gate runs associated with this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gate_runs: Vec<String>,
    /// Version of assay that created this session.
    pub assay_version: String,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "work-session",
        generate: || schemars::schema_for!(WorkSession),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "session-phase",
        generate: || schemars::schema_for!(SessionPhase),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_phase_serializes_as_snake_case() {
        let json = serde_json::to_string(&SessionPhase::AgentRunning).expect("serialize");
        assert_eq!(json, r#""agent_running""#);

        let json = serde_json::to_string(&SessionPhase::GateEvaluated).expect("serialize");
        assert_eq!(json, r#""gate_evaluated""#);
    }

    #[test]
    fn can_transition_happy_path() {
        assert!(SessionPhase::Created.can_transition_to(SessionPhase::AgentRunning));
        assert!(SessionPhase::AgentRunning.can_transition_to(SessionPhase::GateEvaluated));
        assert!(SessionPhase::GateEvaluated.can_transition_to(SessionPhase::Completed));
    }

    #[test]
    fn can_transition_to_abandoned_from_any() {
        assert!(SessionPhase::Created.can_transition_to(SessionPhase::Abandoned));
        assert!(SessionPhase::AgentRunning.can_transition_to(SessionPhase::Abandoned));
        assert!(SessionPhase::GateEvaluated.can_transition_to(SessionPhase::Abandoned));
    }

    #[test]
    fn cannot_transition_from_terminal() {
        for target in [
            SessionPhase::Created,
            SessionPhase::AgentRunning,
            SessionPhase::GateEvaluated,
            SessionPhase::Completed,
            SessionPhase::Abandoned,
        ] {
            assert!(
                !SessionPhase::Completed.can_transition_to(target),
                "Completed should not transition to {target:?}"
            );
            assert!(
                !SessionPhase::Abandoned.can_transition_to(target),
                "Abandoned should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn cannot_skip_phases() {
        assert!(!SessionPhase::Created.can_transition_to(SessionPhase::GateEvaluated));
        assert!(!SessionPhase::Created.can_transition_to(SessionPhase::Completed));
        assert!(!SessionPhase::AgentRunning.can_transition_to(SessionPhase::Completed));
    }

    #[test]
    fn is_terminal() {
        assert!(!SessionPhase::Created.is_terminal());
        assert!(!SessionPhase::AgentRunning.is_terminal());
        assert!(!SessionPhase::GateEvaluated.is_terminal());
        assert!(SessionPhase::Completed.is_terminal());
        assert!(SessionPhase::Abandoned.is_terminal());
    }

    #[test]
    fn work_session_json_round_trip() {
        let session = WorkSession {
            id: "01HTXYZ123456789ABCDEFGH".to_string(),
            spec_name: "auth-flow".to_string(),
            worktree_path: PathBuf::from("/tmp/worktrees/auth-flow"),
            phase: SessionPhase::GateEvaluated,
            transitions: vec![
                PhaseTransition {
                    from: SessionPhase::Created,
                    to: SessionPhase::AgentRunning,
                    timestamp: Utc::now(),
                    trigger: "agent_started".to_string(),
                    notes: Some("Initial agent launch".to_string()),
                },
                PhaseTransition {
                    from: SessionPhase::AgentRunning,
                    to: SessionPhase::GateEvaluated,
                    timestamp: Utc::now(),
                    trigger: "gate_passed".to_string(),
                    notes: None,
                },
            ],
            agent: AgentInvocation {
                spec_name: "auth-flow".to_string(),
                command: "claude --spec auth-flow".to_string(),
                model: Some("claude-sonnet-4-20250514".to_string()),
            },
            gate_runs: vec!["run-001".to_string(), "run-002".to_string()],
            assay_version: "0.4.0".to_string(),
        };

        let json = serde_json::to_string_pretty(&session).expect("serialize");
        let roundtripped: WorkSession = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(session, roundtripped);
    }

    #[test]
    fn work_session_tolerates_unknown_fields() {
        let json = r#"{
            "id": "01HTXYZ",
            "spec_name": "test",
            "worktree_path": "/tmp/wt",
            "phase": "created",
            "transitions": [],
            "agent": { "spec_name": "test", "command": "echo" },
            "assay_version": "0.4.0",
            "some_future_field": true
        }"#;

        let session: WorkSession =
            serde_json::from_str(json).expect("should tolerate unknown fields");
        assert_eq!(session.id, "01HTXYZ");
    }

    #[test]
    fn phase_transition_optional_notes_omitted() {
        let transition = PhaseTransition {
            from: SessionPhase::Created,
            to: SessionPhase::AgentRunning,
            timestamp: Utc::now(),
            trigger: "start".to_string(),
            notes: None,
        };

        let json = serde_json::to_string(&transition).expect("serialize");
        assert!(
            !json.contains("notes"),
            "JSON should omit None notes, got:\n{json}"
        );
    }

    #[test]
    fn gate_runs_omitted_when_empty() {
        let session = WorkSession {
            id: "01HTXYZ".to_string(),
            spec_name: "minimal".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            phase: SessionPhase::Created,
            transitions: vec![],
            agent: AgentInvocation {
                spec_name: "minimal".to_string(),
                command: "echo".to_string(),
                model: None,
            },
            gate_runs: vec![],
            assay_version: "0.4.0".to_string(),
        };

        let json = serde_json::to_string(&session).expect("serialize");
        assert!(
            !json.contains("gate_runs"),
            "JSON should omit empty gate_runs, got:\n{json}"
        );
    }
}
