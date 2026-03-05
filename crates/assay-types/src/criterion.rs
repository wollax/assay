//! Criterion types for defining acceptance criteria on specs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Enforcement;

/// A single acceptance criterion attached to a spec.
///
/// Each criterion has a name, description, and an optional shell command
/// that can verify it programmatically. When `cmd` is `None`, the criterion
/// is evaluated manually (or in future phases, via an agent `prompt` field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    /// Human-readable name for this criterion.
    pub name: String,

    /// Detailed description of what this criterion checks.
    pub description: String,

    /// Optional shell command to verify this criterion.
    /// Omitted from serialized output when `None`.
    // Forward-compatible: a future `prompt` field will support agent-based evaluation.
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
        };

        let toml_str = toml::to_string(&criterion).expect("serialize to TOML");
        assert!(
            toml_str.contains("timeout = 60"),
            "TOML should include timeout, got:\n{toml_str}"
        );

        let roundtripped: Criterion = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(criterion, roundtripped);
    }
}
