//! Criterion types for defining acceptance criteria on specs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Enforcement;

/// The kind of evaluation a criterion uses.
///
/// When set on a criterion, determines how it is evaluated.
/// `AgentReport` means the criterion is evaluated by an agent
/// via structured reasoning rather than a shell command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CriterionKind {
    /// Evaluated by an agent via structured reasoning.
    AgentReport,
}


impl std::fmt::Display for CriterionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentReport => write!(f, "AgentReport"),
        }
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criterion-kind",
        generate: || schemars::schema_for!(CriterionKind),
    }
}

/// A single acceptance criterion attached to a spec.
///
/// Each criterion has a name, description, and an optional shell command
/// that can verify it programmatically. When `kind` is `AgentReport`, the
/// criterion is evaluated by an agent using the `prompt` field for guidance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("kind = \"AgentReport\""),
            "TOML should include kind, got:\n{toml_str}"
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
    fn criterion_missing_required_fields_deser_fails() {
        // name without description should fail
        let toml_str = r#"name = "test""#;
        let result = toml::from_str::<Criterion>(toml_str);
        assert!(
            result.is_err(),
            "criterion without description should fail deserialization"
        );
    }
}
