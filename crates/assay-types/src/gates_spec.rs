//! Gate specification types for directory-based specs.
//!
//! A `GatesSpec` is loaded from `.assay/specs/<feature>/gates.toml` and
//! defines quality gate criteria with optional traceability to requirement
//! IDs from the companion `spec.toml`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enforcement::GateSection;
use crate::precondition::SpecPreconditions;

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

    /// Optional milestone slug this spec belongs to (e.g. `"my-feature"`).
    ///
    /// When set, this spec is associated with a milestone tracked in
    /// `.assay/milestones/<slug>.toml`. Omitted from serialized output when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestone: Option<String>,

    /// Optional ordering hint within the milestone's chunk sequence.
    ///
    /// Lower values sort earlier. Omitted from serialized output when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,

    /// Slug of the parent gate spec this one extends. When set, the parent's
    /// criteria are inherited with own-wins merge semantics.
    ///
    /// Omitted from serialized output when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// Criteria library slugs to include. Criteria from each library are
    /// merged flat into this gate's criteria list.
    ///
    /// Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,

    /// Preconditions that must be met before gate evaluation proceeds.
    ///
    /// Omitted from serialized output when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<SpecPreconditions>,

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
    use crate::criterion::{CriterionKind, When};
    use crate::enforcement::Enforcement;

    #[test]
    fn minimal_gates_spec_toml_roundtrip() {
        let spec = GatesSpec {
            name: "auth-flow".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            milestone: None,
            order: None,
            extends: None,
            include: vec![],
            preconditions: None,
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
                when: When::default(),
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
            when: When::default(),
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
            when: When::default(),
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("agent_report"),
            "TOML should include kind with snake_case type tag, got:\n{toml_str}"
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
            milestone: None,
            order: None,
            extends: None,
            include: vec![],
            preconditions: None,
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
                    when: When::default(),
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
                    when: When::default(),
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
    fn gates_spec_milestone_fields_roundtrip() {
        let toml_str = r#"
name = "auth-flow"
milestone = "my-feature"
order = 2

[[criteria]]
name = "compiles"
description = "Code compiles"
"#;
        let spec: GatesSpec = toml::from_str(toml_str).expect("parse with milestone fields");
        assert_eq!(spec.name, "auth-flow");
        assert_eq!(spec.milestone, Some("my-feature".to_string()));
        assert_eq!(spec.order, Some(2));

        // Roundtrip
        let re_serialized = toml::to_string(&spec).expect("re-serialize");
        let roundtripped: GatesSpec =
            toml::from_str(&re_serialized).expect("roundtrip deserialize");
        assert_eq!(spec, roundtripped);
        assert_eq!(roundtripped.milestone, Some("my-feature".to_string()));
        assert_eq!(roundtripped.order, Some(2));
    }

    #[test]
    fn gates_spec_milestone_fields_absent_from_legacy_toml() {
        let toml_str = r#"
name = "legacy-spec"

[[criteria]]
name = "check"
description = "A check"
"#;
        let spec: GatesSpec = toml::from_str(toml_str).expect("legacy TOML should parse fine");
        assert_eq!(spec.name, "legacy-spec");
        assert!(
            spec.milestone.is_none(),
            "milestone should be None when absent"
        );
        assert!(spec.order.is_none(), "order should be None when absent");
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
            milestone: None,
            order: None,
            extends: None,
            include: vec![],
            preconditions: None,
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
                when: When::default(),
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

    #[test]
    fn example_close_the_loop_gates_toml_parses() {
        let toml_str = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/close-the-loop/gates.toml"
        ))
        .expect("read example gates.toml");
        let spec: GatesSpec = toml::from_str(&toml_str).expect("parse example gates.toml");
        assert_eq!(spec.name, "close-the-loop");
        assert_eq!(spec.criteria.len(), 2);
        assert_eq!(spec.criteria[0].name, "tool-budget");
        assert_eq!(spec.criteria[1].name, "no-tool-errors");
    }

    // ── Composability fields (v0.7.0) ────────────────────────────────────────

    #[test]
    fn gates_spec_with_extends_roundtrip() {
        let toml_str = r#"
name = "child-gate"
extends = "parent-gate"

[[criteria]]
name = "compiles"
description = "Code compiles"
"#;
        let spec: GatesSpec = toml::from_str(toml_str).expect("parse gates spec with extends");
        assert_eq!(spec.extends, Some("parent-gate".to_string()));

        let re_serialized = toml::to_string(&spec).expect("re-serialize");
        let roundtripped: GatesSpec =
            toml::from_str(&re_serialized).expect("roundtrip deserialize");
        assert_eq!(spec, roundtripped);
        assert_eq!(roundtripped.extends, Some("parent-gate".to_string()));
    }

    #[test]
    fn gates_spec_with_include_roundtrip() {
        let toml_str = r#"
name = "full-gate"
include = ["lib-a", "lib-b"]

[[criteria]]
name = "own-check"
description = "An own criterion"
"#;
        let spec: GatesSpec = toml::from_str(toml_str).expect("parse gates spec with include");
        assert_eq!(spec.include, vec!["lib-a", "lib-b"]);

        let re_serialized = toml::to_string(&spec).expect("re-serialize");
        let roundtripped: GatesSpec =
            toml::from_str(&re_serialized).expect("roundtrip deserialize");
        assert_eq!(spec, roundtripped);
        assert_eq!(roundtripped.include, vec!["lib-a", "lib-b"]);
    }

    #[test]
    fn gates_spec_with_extends_and_include_roundtrip() {
        let toml_str = r#"
name = "combined-gate"
extends = "base-gate"
include = ["security-lib", "perf-lib"]

[[criteria]]
name = "specific-check"
description = "A spec-specific criterion"
cmd = "echo ok"
"#;
        let spec: GatesSpec =
            toml::from_str(toml_str).expect("parse gates spec with extends and include");
        assert_eq!(spec.extends, Some("base-gate".to_string()));
        assert_eq!(spec.include, vec!["security-lib", "perf-lib"]);

        let re_serialized = toml::to_string(&spec).expect("re-serialize");
        let roundtripped: GatesSpec =
            toml::from_str(&re_serialized).expect("roundtrip deserialize");
        assert_eq!(spec, roundtripped);
    }

    #[test]
    fn gates_spec_with_preconditions_roundtrip() {
        let toml_str = r#"
name = "guarded-gate"

[preconditions]
requires = ["auth-flow", "db-schema"]
commands = ["docker ps", "pg_isready"]

[[criteria]]
name = "integration-test"
description = "Integration tests pass"
cmd = "cargo test --test integration"
"#;
        let spec: GatesSpec =
            toml::from_str(toml_str).expect("parse gates spec with preconditions");
        let preconds = spec
            .preconditions
            .as_ref()
            .expect("preconditions should be set");
        assert_eq!(preconds.requires, vec!["auth-flow", "db-schema"]);
        assert_eq!(preconds.commands, vec!["docker ps", "pg_isready"]);

        let re_serialized = toml::to_string(&spec).expect("re-serialize");
        let roundtripped: GatesSpec =
            toml::from_str(&re_serialized).expect("roundtrip deserialize");
        assert_eq!(spec, roundtripped);
    }

    #[test]
    fn gates_spec_legacy_toml_without_composability_fields_parses_cleanly() {
        // A pre-v0.7.0 TOML with no extends, include, or preconditions fields.
        let toml_str = r#"
name = "legacy-gate"
description = "A legacy gate spec"

[[criteria]]
name = "compiles"
description = "Code compiles"
cmd = "cargo build"
requirements = ["REQ-FUNC-001"]
"#;
        let spec: GatesSpec =
            toml::from_str(toml_str).expect("pre-v0.7.0 TOML should parse cleanly");
        assert_eq!(spec.name, "legacy-gate");
        assert!(spec.extends.is_none(), "extends should be None");
        assert!(spec.include.is_empty(), "include should be empty");
        assert!(spec.preconditions.is_none(), "preconditions should be None");
        assert_eq!(spec.criteria.len(), 1);
    }

    #[test]
    fn gates_spec_new_fields_omitted_when_default() {
        let spec = GatesSpec {
            name: "minimal".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            milestone: None,
            order: None,
            extends: None,
            include: vec![],
            preconditions: None,
            criteria: vec![GateCriterion {
                name: "check".to_string(),
                description: "A check".to_string(),
                cmd: None,
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
                when: When::default(),
            }],
        };

        let toml_str = toml::to_string(&spec).expect("serialize to TOML");
        assert!(
            !toml_str.contains("extends"),
            "TOML should omit absent extends, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("include"),
            "TOML should omit empty include, got:\n{toml_str}"
        );
        assert!(
            !toml_str.contains("preconditions"),
            "TOML should omit absent preconditions, got:\n{toml_str}"
        );
    }
}
