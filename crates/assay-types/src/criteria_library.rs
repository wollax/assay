//! Criteria library type for reusable, shareable criterion collections.
//!
//! A `CriteriaLibrary` defines a named, versioned collection of [`crate::Criterion`]
//! values that can be included into a [`crate::GatesSpec`] via the `include` field.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A named, versioned collection of criteria that can be included into gate specs.
///
/// Libraries are authored in TOML and referenced by slug from `gates.toml` files
/// via the `include = ["lib-slug"]` field. This allows common criteria (e.g., "must
/// compile", "tests pass") to be defined once and reused across many gate specs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriteriaLibrary {
    /// Unique slug for this library (e.g. `"rust-basics"`).
    pub name: String,

    /// Human-readable description. Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Optional semver-compatible version string (e.g. `"1.0.0"`).
    ///
    /// Stored as a plain string here; semver validation happens in `assay-core`.
    /// Omitted from serialized output when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Tags for categorization and filtering (e.g. `["rust", "build"]`).
    ///
    /// Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Criteria provided by this library.
    pub criteria: Vec<crate::Criterion>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criteria-library",
        generate: || schemars::schema_for!(CriteriaLibrary),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::criterion::When;

    fn minimal_criterion() -> crate::Criterion {
        crate::Criterion {
            name: "compiles".to_string(),
            description: "Code compiles without errors".to_string(),
            cmd: Some("cargo build".to_string()),
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
    fn criteria_library_minimal_roundtrip() {
        let lib = CriteriaLibrary {
            name: "rust-basics".to_string(),
            description: String::new(),
            version: None,
            tags: vec![],
            criteria: vec![minimal_criterion()],
        };

        let toml_str = toml::to_string(&lib).expect("serialize to TOML");
        let roundtripped: CriteriaLibrary =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(lib, roundtripped);
    }

    #[test]
    fn criteria_library_full_roundtrip() {
        let lib = CriteriaLibrary {
            name: "rust-ci".to_string(),
            description: "Standard Rust CI criteria".to_string(),
            version: Some("1.2.0".to_string()),
            tags: vec!["rust".to_string(), "build".to_string(), "test".to_string()],
            criteria: vec![
                minimal_criterion(),
                crate::Criterion {
                    name: "tests-pass".to_string(),
                    description: "All unit tests pass".to_string(),
                    cmd: Some("cargo test".to_string()),
                    path: None,
                    timeout: Some(120),
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec!["REQ-TEST-001".to_string()],
                    when: When::default(),
                },
            ],
        };

        let toml_str = toml::to_string(&lib).expect("serialize to TOML");
        let roundtripped: CriteriaLibrary =
            toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(lib, roundtripped);
        assert_eq!(roundtripped.version, Some("1.2.0".to_string()));
        assert_eq!(roundtripped.tags.len(), 3);
    }

    #[test]
    fn criteria_library_rejects_unknown_fields() {
        let toml_str = r#"
name = "rust-basics"
unknown_field = "oops"

[[criteria]]
name = "compiles"
description = "Code compiles"
"#;
        let err = toml::from_str::<CriteriaLibrary>(toml_str).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown field, got: {msg}"
        );
    }

    #[test]
    fn criteria_library_optional_fields_absent_from_minimal_toml() {
        let toml_str = r#"
name = "minimal-lib"

[[criteria]]
name = "check"
description = "A check"
"#;
        let lib: CriteriaLibrary =
            toml::from_str(toml_str).expect("minimal library should parse fine");
        assert_eq!(lib.name, "minimal-lib");
        assert!(
            lib.description.is_empty(),
            "description should default to empty"
        );
        assert!(lib.version.is_none(), "version should be None when absent");
        assert!(lib.tags.is_empty(), "tags should be empty when absent");
    }
}
