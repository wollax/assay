//! Gate specification types for directory-based specs.
//!
//! A `GatesSpec` is loaded from `.assay/specs/<feature>/gates.toml` and
//! defines quality gate criteria with optional traceability to requirement
//! IDs from the companion `spec.toml`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enforcement::GateSection;

/// Backward-compatible alias: `GateCriterion` is now [`crate::Criterion`].
///
/// Previously a separate struct with an extra `requirements` field.
/// After the criterion-dedup merge, `Criterion` gained `requirements`
/// directly, making the two types identical. This alias preserves API
/// compatibility for existing consumers.
pub type GateCriterion = crate::Criterion;

/// A gate specification loaded from `gates.toml` in a directory-based spec.
///
/// Parallel to [`crate::Spec`] but designed for the directory layout where
/// criteria live in `gates.toml` alongside a `spec.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GatesSpec {
    /// Display name for this gate spec (must match directory name / feature spec name).
    pub name: String,

    /// Human-readable description. Defaults to empty string.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Gate configuration section (enforcement defaults).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateSection>,

    /// Spec slugs this gate spec depends on. Used for dependency ordering and cycle detection.
    /// Each entry is a slug — the spec file name without extension (e.g. `"auth-flow"`
    /// for `specs/auth-flow.toml`). Entries must be unique and non-empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,

    /// Gate criteria that must be satisfied.
    pub criteria: Vec<GateCriterion>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gates-spec",
        generate: || schemars::schema_for!(GatesSpec),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-criterion",
        generate: || schemars::schema_for!(crate::Criterion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::criterion::CriterionKind;
    use crate::enforcement::Enforcement;

    #[test]
    fn minimal_gates_spec_toml_roundtrip() {
        let spec = GatesSpec {
            name: "auth-flow".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![GateCriterion {
                name: "compiles".to_string(),
                description: "Code compiles".to_string(),
                cmd: Some("cargo build".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };

        let toml_str = toml::to_string(&spec).expect("serialize to TOML");
        let roundtripped: GatesSpec = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(spec, roundtripped);
    }

    #[test]
    fn full_gates_spec_toml_roundtrip() {
        let toml_str = r#"
name = "auth-flow"

[[criteria]]
name = "auth-compiles"
description = "Auth module compiles without errors"
cmd = "cargo build -p auth"
requirements = ["REQ-FUNC-001", "REQ-FUNC-002"]

[[criteria]]
name = "password-policy"
description = "Password hashing meets security requirements"
requirements = ["REQ-SEC-001"]
"#;

        let spec: GatesSpec = toml::from_str(toml_str).expect("parse gates spec");

        assert_eq!(spec.name, "auth-flow");
        assert_eq!(spec.criteria.len(), 2);

        let c1 = &spec.criteria[0];
        assert_eq!(c1.name, "auth-compiles");
        assert_eq!(c1.cmd, Some("cargo build -p auth".to_string()));
        assert_eq!(c1.requirements, vec!["REQ-FUNC-001", "REQ-FUNC-002"]);

        let c2 = &spec.criteria[1];
        assert_eq!(c2.name, "password-policy");
        assert_eq!(c2.cmd, None);
        assert_eq!(c2.requirements, vec!["REQ-SEC-001"]);

        // Roundtrip
        let re_serialized = toml::to_string(&spec).expect("re-serialize");
        let roundtripped: GatesSpec =
            toml::from_str(&re_serialized).expect("roundtrip deserialize");
        assert_eq!(spec, roundtripped);
    }

    #[test]
    fn gates_spec_rejects_unknown_fields() {
        let toml_str = r#"
name = "test"
unknown = "oops"

[[criteria]]
name = "c1"
description = "d1"
"#;
        let err = toml::from_str::<GatesSpec>(toml_str).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown field, got: {msg}"
        );
    }

    #[test]
    fn gate_criterion_rejects_unknown_fields() {
        let toml_str = r#"
name = "test"

[[criteria]]
name = "c1"
description = "d1"
unknown_crit_key = true
"#;
        let err = toml::from_str::<GatesSpec>(toml_str).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown criterion key, got: {msg}"
        );
    }

    #[test]
    fn gate_criterion_cmd_none_omitted_in_serialization() {
        let criterion = GateCriterion {
            name: "descriptive".to_string(),
            description: "No command".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
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
        assert!(
            !toml_str.contains("requirements"),
            "TOML should omit empty requirements, got:\n{toml_str}"
        );
    }

    #[test]
    fn gate_criterion_with_timeout_toml_roundtrip() {
        let criterion = GateCriterion {
            name: "slow-test".to_string(),
            description: "A slow integration test".to_string(),
            cmd: Some("cargo test -- --ignored".to_string()),
            path: None,
            timeout: Some(120),
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec!["REQ-FUNC-001".to_string()],
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("timeout = 120"),
            "TOML should include timeout, got:\n{toml_str}"
        );

        let roundtripped: GateCriterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn gate_criterion_agent_report_toml_roundtrip() {
        let criterion = GateCriterion {
            name: "security-review".to_string(),
            description: "Agent reviews for security issues".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: Some(CriterionKind::AgentReport),
            prompt: Some("Check for SQL injection in all DB queries".to_string()),
            requirements: vec!["REQ-SEC-001".to_string()],
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("kind = \"AgentReport\""),
            "TOML should include kind, got:\n{toml_str}"
        );
        assert!(
            toml_str.contains("prompt ="),
            "TOML should include prompt, got:\n{toml_str}"
        );

        let roundtripped: GateCriterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }

    #[test]
    fn gates_spec_with_agent_criterion_roundtrip() {
        let spec = GatesSpec {
            name: "mixed-gates".to_string(),
            description: "Both command and agent criteria".to_string(),
            gate: None,
            depends: vec![],
            criteria: vec![
                GateCriterion {
                    name: "compiles".to_string(),
                    description: "Code compiles".to_string(),
                    cmd: Some("cargo build".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                GateCriterion {
                    name: "architecture-review".to_string(),
                    description: "Agent reviews architecture".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: Some(CriterionKind::AgentReport),
                    prompt: Some("Evaluate module coupling and cohesion".to_string()),
                    requirements: vec!["REQ-ARCH-001".to_string()],
                },
            ],
        };

        let toml_str = toml::to_string(&spec).expect("serialize to TOML");
        let roundtripped: GatesSpec = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(spec, roundtripped);
        assert_eq!(roundtripped.criteria.len(), 2);
        assert!(roundtripped.criteria[0].kind.is_none());
        assert_eq!(
            roundtripped.criteria[1].kind,
            Some(CriterionKind::AgentReport)
        );
    }

    #[test]
    fn gates_spec_gate_section_toml_roundtrip() {
        let spec = GatesSpec {
            name: "gated-spec".to_string(),
            description: "Spec with gate section".to_string(),
            gate: Some(GateSection {
                enforcement: Enforcement::Advisory,
            }),
            depends: vec![],
            criteria: vec![GateCriterion {
                name: "check".to_string(),
                description: "A check".to_string(),
                cmd: Some("echo ok".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };

        let toml_str = toml::to_string(&spec).expect("serialize to TOML");
        assert!(
            toml_str.contains("[gate]"),
            "TOML should include [gate] section, got:\n{toml_str}"
        );
        assert!(
            toml_str.contains("advisory"),
            "TOML should include advisory enforcement, got:\n{toml_str}"
        );

        let roundtripped: GatesSpec = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(spec, roundtripped);
        assert_eq!(
            roundtripped.gate.unwrap().enforcement,
            Enforcement::Advisory
        );
    }
}
