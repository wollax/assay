//! Precondition types for gate spec execution guards.
//!
//! Preconditions declare external requirements that must hold before a gate is
//! evaluated. [`SpecPreconditions`] is the TOML-authored input type (what the
//! spec author writes); [`PreconditionStatus`], [`RequireStatus`], and
//! [`CommandStatus`] are the runtime output types produced during evaluation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Preconditions declared in a `gates.toml` `[preconditions]` section.
///
/// When present, the gate runner checks all `requires` slugs (other gate specs
/// whose last run must have passed) and runs all `commands` before evaluating
/// any criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpecPreconditions {
    /// Slugs of other gate specs that must have passed their last recorded run.
    ///
    /// Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,

    /// Shell commands that must exit 0 before gate evaluation proceeds.
    ///
    /// Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "spec-preconditions",
        generate: || schemars::schema_for!(SpecPreconditions),
    }
}

/// Runtime result of evaluating all preconditions for a gate spec.
///
/// Produced by the gate runner after checking all [`SpecPreconditions`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PreconditionStatus {
    /// Status of each `requires` entry.
    pub requires: Vec<RequireStatus>,

    /// Status of each `commands` entry.
    pub commands: Vec<CommandStatus>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "precondition-status",
        generate: || schemars::schema_for!(PreconditionStatus),
    }
}

/// Status of a single spec slug listed in [`SpecPreconditions::requires`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RequireStatus {
    /// The gate spec slug that was checked.
    pub spec_slug: String,

    /// Whether the last recorded run for this slug passed.
    pub passed: bool,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "require-status",
        generate: || schemars::schema_for!(RequireStatus),
    }
}

/// Status of a single shell command listed in [`SpecPreconditions::commands`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommandStatus {
    /// The shell command that was executed.
    pub command: String,

    /// Whether the command exited with code 0.
    pub passed: bool,

    /// Captured stdout/stderr output. Omitted from serialized output when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "command-status",
        generate: || schemars::schema_for!(CommandStatus),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_preconditions_full_roundtrip() {
        let preconds = SpecPreconditions {
            requires: vec!["auth-flow".to_string(), "db-schema".to_string()],
            commands: vec!["docker ps".to_string(), "pg_isready".to_string()],
        };

        let toml_str = toml::to_string(&preconds).expect("serialize to TOML");
        let roundtripped: SpecPreconditions =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(preconds, roundtripped);
    }

    #[test]
    fn spec_preconditions_minimal_defaults() {
        // Both fields are empty by default
        let preconds = SpecPreconditions {
            requires: vec![],
            commands: vec![],
        };

        let toml_str = toml::to_string(&preconds).expect("serialize to TOML");
        // Empty vecs should be omitted
        assert!(
            !toml_str.contains("requires"),
            "empty requires should be omitted, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("commands"),
            "empty commands should be omitted, got:\n{toml_str}"
        );

        let roundtripped: SpecPreconditions =
            toml::from_str(&toml_str).expect("deserialize empty TOML");
        assert_eq!(preconds, roundtripped);
    }

    #[test]
    fn spec_preconditions_absent_fields_use_defaults() {
        let toml_str = r#"
requires = ["auth-flow"]
"#;
        let preconds: SpecPreconditions =
            toml::from_str(toml_str).expect("parse partial preconditions");
        assert_eq!(preconds.requires, vec!["auth-flow"]);
        assert!(
            preconds.commands.is_empty(),
            "commands should default to empty"
        );
    }

    #[test]
    fn spec_preconditions_rejects_unknown_fields() {
        let toml_str = r#"
requires = ["auth-flow"]
unknown_key = "oops"
"#;
        let err = toml::from_str::<SpecPreconditions>(toml_str).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown field, got: {msg}"
        );
    }

    #[test]
    fn precondition_status_json_roundtrip() {
        let status = PreconditionStatus {
            requires: vec![
                RequireStatus {
                    spec_slug: "auth-flow".to_string(),
                    passed: true,
                },
                RequireStatus {
                    spec_slug: "db-schema".to_string(),
                    passed: false,
                },
            ],
            commands: vec![
                CommandStatus {
                    command: "docker ps".to_string(),
                    passed: true,
                    output: Some("CONTAINER ID   IMAGE\n".to_string()),
                },
                CommandStatus {
                    command: "pg_isready".to_string(),
                    passed: true,
                    output: None,
                },
            ],
        };

        let json = serde_json::to_string(&status).expect("serialize to JSON");
        let roundtripped: PreconditionStatus =
            serde_json::from_str(&json).expect("deserialize from JSON");
        assert_eq!(status, roundtripped);
    }

    #[test]
    fn require_status_json_roundtrip() {
        let status = RequireStatus {
            spec_slug: "auth-flow".to_string(),
            passed: true,
        };
        let json = serde_json::to_string(&status).expect("serialize to JSON");
        let roundtripped: RequireStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, roundtripped);
    }

    #[test]
    fn command_status_output_omitted_when_none() {
        let status = CommandStatus {
            command: "echo ok".to_string(),
            passed: true,
            output: None,
        };

        let json = serde_json::to_string(&status).expect("serialize to JSON");
        assert!(
            !json.contains("output"),
            "output field should be omitted when None, got: {json}"
        );

        let roundtripped: CommandStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, roundtripped);
    }

    #[test]
    fn command_status_output_present_when_some() {
        let status = CommandStatus {
            command: "docker ps".to_string(),
            passed: true,
            output: Some("container-list".to_string()),
        };

        let json = serde_json::to_string(&status).expect("serialize to JSON");
        assert!(
            json.contains("output"),
            "output field should be present when Some, got: {json}"
        );

        let roundtripped: CommandStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, roundtripped);
    }
}
