//! Input/output structs for the gate and criteria wizards.
//!
//! These types are surface-agnostic: the CLI, MCP tools, and TUI all construct
//! them from their own input mechanisms and then call `assay_core::wizard::apply_*`.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{CriteriaLibrary, GatesSpec, SpecPreconditions};

/// Single criterion as the user entered it (not yet a `Criterion` — the wizard
/// validates and promotes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriterionInput {
    /// Human-readable criterion name.
    pub name: String,
    /// Detailed description of what this criterion checks.
    pub description: String,
    /// Optional shell command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
}

/// Full gate wizard input payload. Consumed by `apply_gate_wizard`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateWizardInput {
    /// Slug for the gate spec (used as the spec directory name).
    pub slug: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Slug of the parent gate spec this one extends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    /// Criteria library slugs to include.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
    /// Criteria to add to the gate spec.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<CriterionInput>,
    /// Preconditions that must be met before gate evaluation proceeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<SpecPreconditions>,
    /// If true, overwrite an existing `gates.toml` (edit mode). If false, return
    /// an AlreadyExists error on collision.
    #[serde(default)]
    pub overwrite: bool,
}

/// Output of `apply_gate_wizard`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GateWizardOutput {
    /// Path to the written `gates.toml` file.
    pub path: PathBuf,
    /// The final `GatesSpec` that was written.
    pub spec: GatesSpec,
}

/// Criteria library wizard input. Consumed by `apply_criteria_wizard`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriteriaWizardInput {
    /// Unique slug for this library (used as the library file name).
    pub name: String,
    /// Human-readable description. Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Optional semver-compatible version string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Tags for categorization and filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Criteria to include in the library.
    pub criteria: Vec<CriterionInput>,
    /// If true, overwrite an existing library file (edit mode). If false, return
    /// an AlreadyExists error on collision.
    #[serde(default)]
    pub overwrite: bool,
}

/// Output of `apply_criteria_wizard`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CriteriaWizardOutput {
    /// Path to the written library file.
    pub path: PathBuf,
    /// The final `CriteriaLibrary` that was written.
    pub library: CriteriaLibrary,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: GateWizardInput full payload roundtrip ────────────────────────

    #[test]
    fn gate_wizard_input_full_roundtrip_toml() {
        let input = GateWizardInput {
            slug: "auth-flow".to_string(),
            description: Some("Authentication gate spec".to_string()),
            extends: Some("base-gate".to_string()),
            include: vec!["security-lib".to_string(), "perf-lib".to_string()],
            criteria: vec![
                CriterionInput {
                    name: "compiles".to_string(),
                    description: "Code compiles without errors".to_string(),
                    cmd: Some("cargo build".to_string()),
                },
                CriterionInput {
                    name: "tests-pass".to_string(),
                    description: "All tests pass".to_string(),
                    cmd: None,
                },
            ],
            preconditions: Some(SpecPreconditions {
                requires: vec!["db-schema".to_string()],
                commands: vec!["docker ps".to_string()],
            }),
            overwrite: true,
        };

        let toml_str = toml::to_string(&input).expect("serialize to TOML");
        let roundtripped: GateWizardInput =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(input, roundtripped);
    }

    #[test]
    fn gate_wizard_input_full_roundtrip_json() {
        let input = GateWizardInput {
            slug: "auth-flow".to_string(),
            description: Some("Authentication gate spec".to_string()),
            extends: Some("base-gate".to_string()),
            include: vec!["security-lib".to_string()],
            criteria: vec![CriterionInput {
                name: "compiles".to_string(),
                description: "Code compiles".to_string(),
                cmd: Some("cargo build".to_string()),
            }],
            preconditions: Some(SpecPreconditions {
                requires: vec!["db-schema".to_string()],
                commands: vec![],
            }),
            overwrite: true,
        };

        let json = serde_json::to_string(&input).expect("serialize to JSON");
        let roundtripped: GateWizardInput =
            serde_json::from_str(&json).expect("deserialize from JSON");
        assert_eq!(input, roundtripped);
    }

    // ── Test 2: GateWizardInput minimal payload roundtrip ─────────────────────

    #[test]
    fn gate_wizard_input_minimal_roundtrip_toml() {
        let input = GateWizardInput {
            slug: "minimal-gate".to_string(),
            description: None,
            extends: None,
            include: vec![],
            criteria: vec![],
            preconditions: None,
            overwrite: false,
        };

        let toml_str = toml::to_string(&input).expect("serialize to TOML");
        // Optional fields should be absent from TOML
        assert!(
            !toml_str.contains("description"),
            "description should be omitted when None, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("extends"),
            "extends should be omitted when None, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("include"),
            "include should be omitted when empty, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("criteria"),
            "criteria should be omitted when empty, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("preconditions"),
            "preconditions should be omitted when None, got:\n{toml_str}"
        );

        let roundtripped: GateWizardInput =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(input, roundtripped);
    }

    #[test]
    fn gate_wizard_input_minimal_roundtrip_json() {
        let input = GateWizardInput {
            slug: "minimal-gate".to_string(),
            description: None,
            extends: None,
            include: vec![],
            criteria: vec![],
            preconditions: None,
            overwrite: false,
        };

        let json = serde_json::to_string(&input).expect("serialize to JSON");
        let roundtripped: GateWizardInput =
            serde_json::from_str(&json).expect("deserialize from JSON");
        assert_eq!(input, roundtripped);
    }

    // ── Test 3: GateWizardInput rejects unknown fields ────────────────────────

    #[test]
    fn gate_wizard_input_rejects_unknown_fields_toml() {
        let bad_toml = r#"
slug = "auth-flow"
mystery = "x"
"#;
        let err = toml::from_str::<GateWizardInput>(bad_toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown field, got: {msg}"
        );
    }

    #[test]
    fn gate_wizard_input_rejects_unknown_fields_json() {
        let bad_json = r#"{"slug": "auth-flow", "mystery": "x"}"#;
        let err = serde_json::from_str::<GateWizardInput>(bad_json).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown field, got: {msg}"
        );
    }

    // ── Test 4: CriteriaWizardInput roundtrip ─────────────────────────────────

    #[test]
    fn criteria_wizard_input_full_roundtrip_toml() {
        let input = CriteriaWizardInput {
            name: "rust-ci".to_string(),
            description: "Standard Rust CI criteria".to_string(),
            version: Some("1.0.0".to_string()),
            tags: vec!["rust".to_string(), "build".to_string()],
            criteria: vec![CriterionInput {
                name: "compiles".to_string(),
                description: "Code compiles".to_string(),
                cmd: Some("cargo build".to_string()),
            }],
            overwrite: false,
        };

        let toml_str = toml::to_string(&input).expect("serialize to TOML");
        let roundtripped: CriteriaWizardInput =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(input, roundtripped);
    }

    #[test]
    fn criteria_wizard_input_full_roundtrip_json() {
        let input = CriteriaWizardInput {
            name: "security-checks".to_string(),
            description: "Security criteria library".to_string(),
            version: Some("2.1.0".to_string()),
            tags: vec!["security".to_string()],
            criteria: vec![CriterionInput {
                name: "no-sql-injection".to_string(),
                description: "No SQL injection vulnerabilities".to_string(),
                cmd: None,
            }],
            overwrite: true,
        };

        let json = serde_json::to_string(&input).expect("serialize to JSON");
        let roundtripped: CriteriaWizardInput =
            serde_json::from_str(&json).expect("deserialize from JSON");
        assert_eq!(input, roundtripped);
    }

    // ── Test 5: CriterionInput (re-homed) roundtrip ───────────────────────────

    #[test]
    fn criterion_input_roundtrip_with_cmd() {
        let input = CriterionInput {
            name: "compiles".to_string(),
            description: "Code compiles without errors".to_string(),
            cmd: Some("cargo build".to_string()),
        };

        let toml_str = toml::to_string(&input).expect("serialize to TOML");
        let roundtripped: CriterionInput =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(input, roundtripped);
    }

    #[test]
    fn criterion_input_roundtrip_without_cmd() {
        // Old payloads with only name + description + no cmd should still deserialize
        let toml_str = r#"
name = "code-review"
description = "Agent reviews code quality"
"#;
        let input: CriterionInput = toml::from_str(toml_str).expect("deserialize from TOML");
        assert_eq!(input.name, "code-review");
        assert_eq!(input.description, "Agent reviews code quality");
        assert!(input.cmd.is_none(), "cmd should default to None");
    }

    #[test]
    fn criterion_input_cmd_omitted_when_none() {
        let input = CriterionInput {
            name: "review".to_string(),
            description: "Review code".to_string(),
            cmd: None,
        };

        let toml_str = toml::to_string(&input).expect("serialize to TOML");
        assert!(
            !toml_str.contains("cmd"),
            "cmd should be omitted when None, got:\n{toml_str}"
        );

        let json = serde_json::to_string(&input).expect("serialize to JSON");
        assert!(
            !json.contains("cmd"),
            "cmd should be omitted from JSON when None, got: {json}"
        );
    }

    // ── Test 6: JsonSchema derive for GateWizardInput ─────────────────────────

    #[test]
    fn gate_wizard_input_schema_is_non_empty() {
        let schema = schemars::schema_for!(GateWizardInput);
        let schema_str = serde_json::to_string(&schema).expect("serialize schema to JSON");
        assert!(
            !schema_str.is_empty(),
            "GateWizardInput schema should be non-empty"
        );
        assert!(
            schema_str.contains("slug"),
            "schema should reference 'slug' field, got: {schema_str}"
        );
    }

    // ── Test 7: JsonSchema derive for CriteriaWizardInput ─────────────────────

    #[test]
    fn criteria_wizard_input_schema_is_non_empty() {
        let schema = schemars::schema_for!(CriteriaWizardInput);
        let schema_str = serde_json::to_string(&schema).expect("serialize schema to JSON");
        assert!(
            !schema_str.is_empty(),
            "CriteriaWizardInput schema should be non-empty"
        );
        assert!(
            schema_str.contains("name"),
            "schema should reference 'name' field, got: {schema_str}"
        );
    }
}
