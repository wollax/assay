//! Criterion types for defining acceptance criteria on specs.

use std::num::NonZeroU32;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Enforcement;

/// The kind of evaluation a criterion uses.
///
/// When set on a criterion, determines how it is evaluated.
/// `AgentReport` means the criterion is evaluated by an agent
/// via structured reasoning rather than a shell command.
///
/// Serializes using an internally tagged format with `type` as the tag key,
/// using `snake_case` for variant names (e.g. `{"type":"agent_report"}`).
/// Old PascalCase format (`{"type":"AgentReport"}`) is accepted via aliases
/// for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CriterionKind {
    /// Evaluated by an agent via structured reasoning.
    #[serde(alias = "AgentReport")]
    AgentReport,

    /// Criterion evaluated by counting events of a given type in the agent
    /// session log. Parameters mirror `GateKind::EventCount`.
    #[serde(alias = "EventCount")]
    EventCount {
        /// The event type to count, matching an `AgentEvent` serde tag.
        event_type: String,
        /// Inclusive lower bound on the matched event count. `None` means no lower bound.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<u32>,
        /// Inclusive upper bound on the matched event count. `None` means no upper bound.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },

    /// Criterion that passes only when no tool returned an error during the session.
    #[serde(alias = "NoToolErrors")]
    NoToolErrors,
}

impl std::fmt::Display for CriterionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentReport => write!(f, "AgentReport"),
            Self::EventCount { .. } => write!(f, "EventCount"),
            Self::NoToolErrors => write!(f, "NoToolErrors"),
        }
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criterion-kind",
        generate: || schemars::schema_for!(CriterionKind),
    }
}

/// Declares when a gate criterion is evaluated.
///
/// Spec authors use this to designate cheap event-based criteria as
/// mid-session checkpoints while keeping expensive command/file gates
/// at session end.
///
/// # Warning on command/file gates at checkpoints
///
/// Using `AfterToolCalls` or `OnEvent` with a `cmd`- or `path`-based
/// criterion evaluates against a **partial working directory**. Prefer
/// `EventCount` and `NoToolErrors` criteria for checkpoints.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum When {
    /// Evaluate at the end of the session (default behavior).
    #[default]
    SessionEnd,
    /// Evaluate when the agent has emitted exactly N total tool calls.
    /// The pipeline fires this trigger once when the cumulative tool-call
    /// count reaches N.
    AfterToolCalls {
        /// Tool-call threshold at which to evaluate. Enforced non-zero at
        /// both the type level (`NonZeroU32`) and the serde boundary.
        n: NonZeroU32,
    },
    /// Evaluate when the most-recent event in the stream has a serde
    /// type tag matching `event_type` (e.g. `"tool_called"`).
    OnEvent {
        /// Event type tag to match (matches `AgentEvent` serde tag).
        event_type: String,
    },
}

impl When {
    /// Returns `true` if this is `When::SessionEnd`.
    pub fn is_session_end(&self) -> bool {
        matches!(self, Self::SessionEnd)
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "when",
        generate: || schemars::schema_for!(When),
    }
}

/// A single acceptance criterion attached to a spec.
///
/// Each criterion has a name, description, and an optional shell command
/// that can verify it programmatically. When `kind` is `AgentReport`, the
/// criterion is evaluated by an agent using the `prompt` field for guidance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    /// Human-readable name for this criterion.
    pub name: String,

    /// Detailed description of what this criterion checks.
    pub description: String,

    /// Optional shell command to verify this criterion.
    /// Omitted from serialized output when `None`.
    /// Mutually exclusive with `kind = AgentReport`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cmd: Option<String>,

    /// Optional file path to check for existence.
    /// When set (and `cmd` is `None`), the criterion evaluates as a `FileExists` gate.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,

    /// Optional timeout in seconds for this criterion's command.
    /// Overrides the global default but is overridden by CLI `--timeout`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout: Option<u64>,

    /// Enforcement level override for this criterion. `None` means "use the
    /// spec-level default from `[gate]` section" (which itself defaults to `required`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub enforcement: Option<Enforcement>,

    /// Criterion evaluation kind. When set to `AgentReport`, this criterion
    /// is evaluated by an agent (not a shell command). Mutually exclusive
    /// with `cmd` and `path`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<CriterionKind>,

    /// Instruction prompt for agent-evaluated criteria.
    /// Provides guidance to the agent on what to evaluate.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prompt: Option<String>,

    /// Requirement IDs this criterion traces to (e.g., `["REQ-FUNC-001"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,

    /// Declares when this criterion is evaluated. Defaults to `When::SessionEnd`
    /// (evaluation at session end). When set to `When::AfterToolCalls { .. }` or
    /// `When::OnEvent { .. }`, the criterion becomes a mid-session checkpoint.
    #[serde(default, skip_serializing_if = "When::is_session_end")]
    pub when: When,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criterion",
        generate: || schemars::schema_for!(Criterion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criterion_cmd_none_is_valid() {
        let criterion = Criterion {
            name: "builds cleanly".to_string(),
            description: "The project compiles without warnings".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            !toml_str.contains("cmd"),
            "TOML should omit absent cmd, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("timeout"),
            "TOML should omit absent timeout, got:\n{toml_str}"
        );

        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn criterion_cmd_some_is_valid() {
        let criterion = Criterion {
            name: "tests pass".to_string(),
            description: "All unit tests pass".to_string(),
            cmd: Some("cargo test".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains(r#"cmd = "cargo test""#),
            "TOML should include cmd, got:\n{toml_str}"
        );

        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn criterion_with_timeout_toml_roundtrip() {
        let criterion = Criterion {
            name: "long test".to_string(),
            description: "A slow integration test".to_string(),
            cmd: Some("cargo test -- --ignored".to_string()),
            path: None,
            timeout: Some(60),
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("timeout = 60"),
            "TOML should include timeout, got:\n{toml_str}"
        );

        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn criterion_agent_report_toml_roundtrip() {
        let criterion = Criterion {
            name: "code-review".to_string(),
            description: "Agent reviews code for security issues".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: Some(CriterionKind::AgentReport),
            prompt: Some("Review the auth module for SQL injection vulnerabilities".to_string()),
            requirements: vec![],
            when: When::default(),
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("agent_report"),
            "TOML should include kind with snake_case type tag, got:\n{toml_str}"
        );
        assert!(
            toml_str.contains("prompt = \"Review"),
            "TOML should include prompt, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("cmd"),
            "TOML should omit cmd for agent criterion, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("path"),
            "TOML should omit path for agent criterion, got:\n{toml_str}"
        );

        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn criterion_kind_omitted_when_none() {
        let criterion = Criterion {
            name: "basic".to_string(),
            description: "A basic criterion".to_string(),
            cmd: Some("echo ok".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            !toml_str.contains("kind"),
            "TOML should omit absent kind, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("prompt"),
            "TOML should omit absent prompt, got:\n{toml_str}"
        );
    }

    #[test]
    fn criterion_enforcement_toml_roundtrip() {
        let criterion = Criterion {
            name: "enforced".to_string(),
            description: "Criterion with enforcement".to_string(),
            cmd: Some("echo ok".to_string()),
            path: None,
            timeout: None,
            enforcement: Some(Enforcement::Advisory),
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("enforcement"),
            "TOML should include enforcement, got:\n{toml_str}"
        );
        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn when_defaults_to_session_end() {
        assert_eq!(When::default(), When::SessionEnd);
    }

    #[test]
    fn when_serde_tagged_roundtrip() {
        use serde_json::json;
        // SessionEnd
        let w = When::SessionEnd;
        let v = serde_json::to_value(&w).unwrap();
        assert_eq!(v, json!({ "type": "session_end" }));
        let back: When = serde_json::from_value(v).unwrap();
        assert_eq!(back, When::SessionEnd);

        // AfterToolCalls { n: NonZeroU32::new(2).unwrap() }
        let w = When::AfterToolCalls {
            n: NonZeroU32::new(2).unwrap(),
        };
        let v = serde_json::to_value(&w).unwrap();
        assert_eq!(v, json!({ "type": "after_tool_calls", "n": 2 }));
        let back: When = serde_json::from_value(v).unwrap();
        assert_eq!(
            back,
            When::AfterToolCalls {
                n: NonZeroU32::new(2).unwrap()
            }
        );

        // OnEvent { event_type: "tool_called" }
        let w = When::OnEvent {
            event_type: "tool_called".to_string(),
        };
        let v = serde_json::to_value(&w).unwrap();
        assert_eq!(
            v,
            json!({ "type": "on_event", "event_type": "tool_called" })
        );
        let back: When = serde_json::from_value(v).unwrap();
        assert_eq!(
            back,
            When::OnEvent {
                event_type: "tool_called".to_string()
            }
        );
    }

    #[test]
    fn criterion_when_omitted_when_session_end() {
        let criterion = Criterion {
            name: "basic".to_string(),
            description: "plain".to_string(),
            cmd: Some("echo ok".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::SessionEnd,
        };
        let toml_str = toml::to_string(&criterion).expect("serialize");
        assert!(
            !toml_str.contains("when"),
            "TOML should omit when=SessionEnd, got:\n{toml_str}"
        );
        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn criterion_when_after_tool_calls_roundtrip() {
        let criterion = Criterion {
            name: "event-check".to_string(),
            description: "checkpoint criterion".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: Some(CriterionKind::NoToolErrors),
            prompt: None,
            requirements: vec![],
            when: When::AfterToolCalls {
                n: NonZeroU32::new(3).unwrap(),
            },
        };
        let toml_str = toml::to_string(&criterion).expect("serialize");
        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(criterion, roundtripped);
        assert_eq!(
            roundtripped.when,
            When::AfterToolCalls {
                n: NonZeroU32::new(3).unwrap()
            }
        );
    }

    #[test]
    fn criterion_when_roundtrip_pre_m024_fixture() {
        // Real pre-M024 fixture: .assay/specs/self-check.toml. Embedded inline
        // to keep this test independent of workspace layout. The goal is to
        // prove that a spec with no `when` field anywhere still deserializes,
        // serializes back without introducing `when`, and round-trips to an
        // equal struct (D028: zero-regression on pre-M024 fixtures).
        let fixture = r#"name = "self-check"
description = "Assay's own quality gates — dogfooding spec"

[gate]
enforcement = "required"

[[criteria]]
name = "formatting"
description = "Code is formatted with rustfmt"
cmd = "cargo fmt --check"

[[criteria]]
name = "linting"
description = "No clippy warnings"
cmd = "cargo clippy --workspace -- -D warnings"

[[criteria]]
name = "tests"
description = "All tests pass"
cmd = "cargo test --workspace"
"#;

        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        struct TinySpec {
            name: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            gate: Option<crate::GateSection>,
            criteria: Vec<Criterion>,
        }

        let parsed: TinySpec = toml::from_str(fixture).expect("parse fixture");
        // All criteria must have when == When::SessionEnd (backward compat — omitted `when` defaults to SessionEnd).
        for c in &parsed.criteria {
            assert_eq!(
                c.when,
                When::SessionEnd,
                "pre-M024 fixture must deserialize omitted when as SessionEnd"
            );
        }
        // Re-serialize: output must not contain the string "when".
        let out = toml::to_string(&parsed).expect("serialize back");
        assert!(
            !out.contains("when"),
            "serialized output should omit `when` field, got:\n{out}"
        );
        // Round-trip must produce an equal struct.
        let reparsed: TinySpec = toml::from_str(&out).expect("reparse");
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn criterion_missing_required_fields_deser_fails() {
        // name without description should fail
        let toml_str = r#"name = "test""#;
        let result = toml::from_str::<Criterion>(toml_str);
        assert!(
            result.is_err(),
            "criterion without description should fail deserialization"
        );
    }

    #[test]
    fn when_after_tool_calls_zero_rejected() {
        let json = r#"{"type":"after_tool_calls","n":0}"#;
        let result = serde_json::from_str::<When>(json);
        assert!(
            result.is_err(),
            "AfterToolCalls n=0 should fail deserialization"
        );
    }

    #[test]
    fn when_after_tool_calls_one_accepted() {
        let json = r#"{"type":"after_tool_calls","n":1}"#;
        let result = serde_json::from_str::<When>(json);
        assert!(
            result.is_ok(),
            "AfterToolCalls n=1 should succeed, got: {:?}",
            result
        );
        assert_eq!(
            result.unwrap(),
            When::AfterToolCalls {
                n: NonZeroU32::new(1).unwrap()
            }
        );
    }

    #[test]
    fn when_is_session_end_method() {
        assert!(When::SessionEnd.is_session_end());
        assert!(
            !When::AfterToolCalls {
                n: NonZeroU32::new(1).unwrap()
            }
            .is_session_end()
        );
        assert!(
            !When::OnEvent {
                event_type: "x".to_string()
            }
            .is_session_end()
        );
    }

    // ── CriterionKind internally tagged serde ──────────────────────────

    #[test]
    fn criterion_kind_internally_tagged_roundtrip() {
        use serde_json::json;

        // AgentReport → {"type":"agent_report"}
        let v = serde_json::to_value(&CriterionKind::AgentReport).unwrap();
        assert_eq!(v, json!({"type": "agent_report"}));
        let back: CriterionKind = serde_json::from_value(v).unwrap();
        assert_eq!(back, CriterionKind::AgentReport);

        // NoToolErrors → {"type":"no_tool_errors"}
        let v = serde_json::to_value(&CriterionKind::NoToolErrors).unwrap();
        assert_eq!(v, json!({"type": "no_tool_errors"}));
        let back: CriterionKind = serde_json::from_value(v).unwrap();
        assert_eq!(back, CriterionKind::NoToolErrors);
    }

    #[test]
    fn criterion_kind_alias_roundtrip() {
        // Old PascalCase format must still deserialize via alias.
        let agent_report: CriterionKind =
            serde_json::from_str(r#"{"type":"AgentReport"}"#).unwrap();
        assert_eq!(agent_report, CriterionKind::AgentReport);

        let no_tool_errors: CriterionKind =
            serde_json::from_str(r#"{"type":"NoToolErrors"}"#).unwrap();
        assert_eq!(no_tool_errors, CriterionKind::NoToolErrors);
    }

    #[test]
    fn criterion_kind_event_count_roundtrip() {
        use serde_json::json;

        // New snake_case format
        let kind = CriterionKind::EventCount {
            event_type: "tool_called".to_string(),
            min: Some(1),
            max: Some(5),
        };
        let v = serde_json::to_value(&kind).unwrap();
        assert_eq!(
            v,
            json!({"type": "event_count", "event_type": "tool_called", "min": 1, "max": 5})
        );
        let back: CriterionKind = serde_json::from_value(v).unwrap();
        assert_eq!(back, kind);

        // Old PascalCase alias format
        let old_format: CriterionKind = serde_json::from_str(
            r#"{"type":"EventCount","event_type":"tool_called","min":1,"max":5}"#,
        )
        .unwrap();
        assert_eq!(old_format, kind);
    }

    #[test]
    fn criterion_kind_toml_pascal_case_alias_roundtrip() {
        // Old TOML with PascalCase variant in internally-tagged format.
        // This is the format that `#[serde(alias)]` preserves for backward compat.
        let toml_agent: CriterionKind =
            toml::from_str(r#"type = "AgentReport""#).expect("AgentReport alias");
        assert_eq!(toml_agent, CriterionKind::AgentReport);

        let toml_no_errors: CriterionKind =
            toml::from_str(r#"type = "NoToolErrors""#).expect("NoToolErrors alias");
        assert_eq!(toml_no_errors, CriterionKind::NoToolErrors);

        let toml_event: CriterionKind = toml::from_str(
            r#"type = "EventCount"
event_type = "tool_called"
min = 1
"#,
        )
        .expect("EventCount alias");
        assert_eq!(
            toml_event,
            CriterionKind::EventCount {
                event_type: "tool_called".to_string(),
                min: Some(1),
                max: None,
            }
        );
    }
}
