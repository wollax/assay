//! Session and evaluation types for agent gate recording.
//!
//! These types support crash-recoverable agent sessions and structured
//! evaluation results (evidence, reasoning, confidence) for agent-reported gates.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enforcement::Enforcement;
use crate::gate_run::CriterionResult;

/// The role of the evaluator who produced a gate result.
///
/// Determines trust level and audit requirements:
/// - `SelfEval` ("self"): the coding agent evaluates its own work (default for v0.2).
/// - `Independent`: a separate agent evaluates (planned for v0.3).
/// - `Human`: a human performed the evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EvaluatorRole {
    /// The coding agent evaluates its own work.
    #[serde(rename = "self")]
    SelfEval,
    /// A separate, independent agent evaluates.
    Independent,
    /// A human performed the evaluation.
    Human,
}


impl std::fmt::Display for EvaluatorRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfEval => write!(f, "self"),
            Self::Independent => write!(f, "independent"),
            Self::Human => write!(f, "human"),
        }
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "evaluator-role",
        generate: || schemars::schema_for!(EvaluatorRole),
    }
}

/// Agent confidence level in an evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// High confidence — clear evidence supports the conclusion.
    High,
    /// Medium confidence — evidence is suggestive but not conclusive.
    Medium,
    /// Low confidence — uncertain; evaluation may need human review.
    Low,
}


impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "confidence",
        generate: || schemars::schema_for!(Confidence),
    }
}

/// A structured evaluation produced by an agent for a single criterion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentEvaluation {
    /// Whether the criterion passed according to this evaluation.
    pub passed: bool,
    /// Concrete facts the agent observed (what was seen).
    pub evidence: String,
    /// Why those facts lead to pass/fail (the agent's reasoning chain).
    pub reasoning: String,
    /// Agent's confidence level in the evaluation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub confidence: Option<Confidence>,
    /// Role of the evaluator who produced this evaluation.
    pub evaluator_role: EvaluatorRole,
    /// When this evaluation was produced.
    pub timestamp: DateTime<Utc>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "agent-evaluation",
        generate: || schemars::schema_for!(AgentEvaluation),
    }
}

/// A crash-recoverable agent session for gate evaluation.
///
/// Tracks in-progress evaluations so that a session can be resumed
/// after an unexpected interruption without losing completed work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Name of the spec being evaluated.
    pub spec_name: String,
    /// When this session was created.
    pub created_at: DateTime<Utc>,
    /// Results from deterministic (command-based) criterion evaluations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_results: Vec<CriterionResult>,
    /// Agent evaluations keyed by criterion name. Multiple evaluations
    /// per criterion are allowed (e.g., re-evaluation after fix).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agent_evaluations: HashMap<String, Vec<AgentEvaluation>>,
    /// Names of all criteria in the spec (for completeness tracking).
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub criteria_names: HashSet<String>,
    /// Resolved enforcement level per criterion name.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub spec_enforcement: HashMap<String, Enforcement>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "agent-session",
        generate: || schemars::schema_for!(AgentSession),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluator_role_self_eval_serializes_as_self() {
        let json = serde_json::to_string(&EvaluatorRole::SelfEval).expect("serialize");
        assert_eq!(json, r#""self""#);

        let roundtripped: EvaluatorRole = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtripped, EvaluatorRole::SelfEval);
    }

    #[test]
    fn evaluator_role_independent_serializes_correctly() {
        let json = serde_json::to_string(&EvaluatorRole::Independent).expect("serialize");
        assert_eq!(json, r#""independent""#);

        let roundtripped: EvaluatorRole = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtripped, EvaluatorRole::Independent);
    }

    #[test]
    fn evaluator_role_human_serializes_correctly() {
        let json = serde_json::to_string(&EvaluatorRole::Human).expect("serialize");
        assert_eq!(json, r#""human""#);

        let roundtripped: EvaluatorRole = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtripped, EvaluatorRole::Human);
    }

    #[test]
    fn confidence_high_serializes_as_high() {
        let json = serde_json::to_string(&Confidence::High).expect("serialize");
        assert_eq!(json, r#""high""#);

        let roundtripped: Confidence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtripped, Confidence::High);
    }

    #[test]
    fn confidence_medium_serializes_correctly() {
        let json = serde_json::to_string(&Confidence::Medium).expect("serialize");
        assert_eq!(json, r#""medium""#);
    }

    #[test]
    fn confidence_low_serializes_correctly() {
        let json = serde_json::to_string(&Confidence::Low).expect("serialize");
        assert_eq!(json, r#""low""#);
    }

    #[test]
    fn agent_evaluation_json_roundtrip() {
        let eval = AgentEvaluation {
            passed: true,
            evidence: "Found auth module with JWT validation".to_string(),
            reasoning: "JWT validation present and tests pass".to_string(),
            confidence: Some(Confidence::High),
            evaluator_role: EvaluatorRole::SelfEval,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&eval).expect("serialize");
        let roundtripped: AgentEvaluation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(eval, roundtripped);
    }

    #[test]
    fn agent_evaluation_omits_confidence_when_none() {
        let eval = AgentEvaluation {
            passed: false,
            evidence: "No tests found".to_string(),
            reasoning: "Directory scan returned zero test files".to_string(),
            confidence: None,
            evaluator_role: EvaluatorRole::Independent,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&eval).expect("serialize");
        assert!(
            !json.contains("confidence"),
            "JSON should omit None confidence, got:\n{json}"
        );
    }

    #[test]
    fn agent_session_json_roundtrip() {
        let mut evaluations = HashMap::new();
        evaluations.insert(
            "code-compiles".to_string(),
            vec![AgentEvaluation {
                passed: true,
                evidence: "cargo build succeeded".to_string(),
                reasoning: "Clean build with no errors".to_string(),
                confidence: Some(Confidence::High),
                evaluator_role: EvaluatorRole::SelfEval,
                timestamp: Utc::now(),
            }],
        );

        let mut criteria_names = HashSet::new();
        criteria_names.insert("code-compiles".to_string());
        criteria_names.insert("tests-pass".to_string());

        let mut spec_enforcement = HashMap::new();
        spec_enforcement.insert("code-compiles".to_string(), Enforcement::Required);
        spec_enforcement.insert("tests-pass".to_string(), Enforcement::Advisory);

        let session = AgentSession {
            session_id: "20260305T200000Z-a1b2c3".to_string(),
            spec_name: "auth-flow".to_string(),
            created_at: Utc::now(),
            command_results: vec![],
            agent_evaluations: evaluations,
            criteria_names,
            spec_enforcement,
        };

        let json = serde_json::to_string_pretty(&session).expect("serialize");
        let roundtripped: AgentSession = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(session, roundtripped);
    }

    #[test]
    fn agent_session_empty_collections_omitted() {
        let session = AgentSession {
            session_id: "test-session".to_string(),
            spec_name: "minimal".to_string(),
            created_at: Utc::now(),
            command_results: vec![],
            agent_evaluations: HashMap::new(),
            criteria_names: HashSet::new(),
            spec_enforcement: HashMap::new(),
        };

        let json = serde_json::to_string(&session).expect("serialize");
        assert!(
            !json.contains("command_results"),
            "JSON should omit empty command_results, got:\n{json}"
        );
        assert!(
            !json.contains("agent_evaluations"),
            "JSON should omit empty agent_evaluations, got:\n{json}"
        );
        assert!(
            !json.contains("criteria_names"),
            "JSON should omit empty criteria_names, got:\n{json}"
        );
        assert!(
            !json.contains("spec_enforcement"),
            "JSON should omit empty spec_enforcement, got:\n{json}"
        );
    }
}
