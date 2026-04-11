//! Resolved gate types for post-composition gate representations.
//!
//! These types are produced by the resolution phase (`assay-core::spec::compose`)
//! and represent a fully expanded gate: own criteria, inherited parent criteria,
//! and criteria merged in from criteria libraries.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Criterion;

/// Identifies where a criterion in a [`ResolvedGate`] originated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CriterionSource {
    /// Criterion is defined directly on this gate.
    Own,
    /// Criterion was inherited from a parent gate via `extends`.
    Parent {
        /// The slug of the parent gate.
        gate_slug: String,
    },
    /// Criterion was merged in from a criteria library via `include`.
    Library {
        /// The slug of the criteria library.
        slug: String,
    },
}

/// A criterion together with its origin in the resolved gate.
///
/// Note: No `#[serde(deny_unknown_fields)]` — this is a runtime output type,
/// not a TOML-authored type, so forward-compatibility is preferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedCriterion {
    /// The criterion definition.
    pub criterion: Criterion,
    /// Where this criterion originated (own, parent, or library).
    pub source: CriterionSource,
}

/// A fully resolved gate: all criteria flattened with their origins recorded.
///
/// Produced by `assay-core::spec::compose::resolve()` after expanding `extends`
/// and `include` references. Suitable for gate evaluation without further I/O.
///
/// Note: No `#[serde(deny_unknown_fields)]` — this is a runtime output type,
/// not a TOML-authored type, so forward-compatibility is preferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedGate {
    /// The slug of this gate (directory name).
    pub gate_slug: String,
    /// The slug of the parent gate, if any (`extends` field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_slug: Option<String>,
    /// Slugs of all criteria libraries included via `include`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub included_libraries: Vec<String>,
    /// All criteria in merge order (parent first, then libraries, then own).
    pub criteria: Vec<ResolvedCriterion>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "resolved-gate",
        generate: || schemars::schema_for!(ResolvedGate),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_criterion(name: &str) -> Criterion {
        use crate::criterion::When;
        Criterion {
            name: name.to_string(),
            description: format!("{name} description"),
            cmd: Some(format!("echo {name}")),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
        }
    }

    #[test]
    fn criterion_source_own_roundtrip() {
        let src = CriterionSource::Own;
        let json = serde_json::to_string(&src).expect("serialize Own");
        assert_eq!(json, r#""own""#);
        let back: CriterionSource = serde_json::from_str(&json).expect("deserialize Own");
        assert_eq!(back, src);
    }

    #[test]
    fn criterion_source_parent_roundtrip() {
        let src = CriterionSource::Parent {
            gate_slug: "base-gate".to_string(),
        };
        let json = serde_json::to_string(&src).expect("serialize Parent");
        let back: CriterionSource = serde_json::from_str(&json).expect("deserialize Parent");
        assert_eq!(back, src);
        assert!(
            json.contains("base-gate"),
            "should contain slug, got: {json}"
        );
    }

    #[test]
    fn criterion_source_library_roundtrip() {
        let src = CriterionSource::Library {
            slug: "rust-basics".to_string(),
        };
        let json = serde_json::to_string(&src).expect("serialize Library");
        let back: CriterionSource = serde_json::from_str(&json).expect("deserialize Library");
        assert_eq!(back, src);
        assert!(
            json.contains("rust-basics"),
            "should contain slug, got: {json}"
        );
    }

    #[test]
    fn resolved_criterion_roundtrip() {
        let rc = ResolvedCriterion {
            criterion: make_criterion("compiles"),
            source: CriterionSource::Own,
        };
        let json = serde_json::to_string(&rc).expect("serialize ResolvedCriterion");
        let back: ResolvedCriterion =
            serde_json::from_str(&json).expect("deserialize ResolvedCriterion");
        assert_eq!(back, rc);
    }

    #[test]
    fn resolved_gate_minimal_roundtrip() {
        let gate = ResolvedGate {
            gate_slug: "my-spec".to_string(),
            parent_slug: None,
            included_libraries: vec![],
            criteria: vec![ResolvedCriterion {
                criterion: make_criterion("passes"),
                source: CriterionSource::Own,
            }],
        };
        let json = serde_json::to_string(&gate).expect("serialize ResolvedGate");
        let back: ResolvedGate = serde_json::from_str(&json).expect("deserialize ResolvedGate");
        assert_eq!(back, gate);
        // Optional fields omitted when empty
        assert!(
            !json.contains("parent_slug"),
            "None should be omitted: {json}"
        );
        assert!(
            !json.contains("included_libraries"),
            "empty vec should be omitted: {json}"
        );
    }

    #[test]
    fn resolved_gate_full_roundtrip() {
        let gate = ResolvedGate {
            gate_slug: "child-spec".to_string(),
            parent_slug: Some("parent-spec".to_string()),
            included_libraries: vec!["rust-basics".to_string(), "security".to_string()],
            criteria: vec![
                ResolvedCriterion {
                    criterion: make_criterion("inherited"),
                    source: CriterionSource::Parent {
                        gate_slug: "parent-spec".to_string(),
                    },
                },
                ResolvedCriterion {
                    criterion: make_criterion("from-lib"),
                    source: CriterionSource::Library {
                        slug: "rust-basics".to_string(),
                    },
                },
                ResolvedCriterion {
                    criterion: make_criterion("own"),
                    source: CriterionSource::Own,
                },
            ],
        };
        let json = serde_json::to_string(&gate).expect("serialize ResolvedGate full");
        let back: ResolvedGate =
            serde_json::from_str(&json).expect("deserialize ResolvedGate full");
        assert_eq!(back, gate);
        assert_eq!(back.criteria.len(), 3);
        assert_eq!(back.parent_slug, Some("parent-spec".to_string()));
        assert_eq!(back.included_libraries.len(), 2);
    }

    #[test]
    fn resolved_gate_schema_registered() {
        // Verify the schema entry is registered via inventory
        use crate::schema_registry;
        let names: Vec<&str> = schema_registry::all_entries().map(|e| e.name).collect();
        assert!(
            names.contains(&"resolved-gate"),
            "resolved-gate should be registered, found: {names:?}"
        );
    }
}
