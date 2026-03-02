//! Configuration loading and validation.
//!
//! Handles reading, parsing, and validating Assay project configuration
//! from files and environment.

use std::fmt;

use assay_types::Config;

/// A single validation issue in a config file.
#[derive(Debug, Clone)]
pub struct ConfigError {
    /// The field path (e.g., "project_name", "[gates].default_timeout").
    pub field: String,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Parse a config from a TOML string without validation.
///
/// Returns the raw `toml::de::Error` on failure, preserving line/column
/// information. Callers that need validation should use [`load()`] instead.
pub fn from_str(s: &str) -> std::result::Result<Config, toml::de::Error> {
    toml::from_str(s)
}

#[cfg(test)]
mod tests {
    // Config and GatesConfig accessed via from_str return type and field access.

    // ── from_str tests ──────────────────────────────────────────────

    #[test]
    fn from_str_valid_all_fields() {
        let toml = r#"
project_name = "my-project"
specs_dir = "my-specs/"

[gates]
default_timeout = 600
working_dir = "/tmp"
"#;
        let config = super::from_str(toml).expect("valid TOML should parse");

        assert_eq!(config.project_name, "my-project");
        assert_eq!(config.specs_dir, "my-specs/");
        let gates = config.gates.expect("gates should be Some");
        assert_eq!(gates.default_timeout, 600);
        assert_eq!(gates.working_dir.as_deref(), Some("/tmp"));
    }

    #[test]
    fn from_str_minimal_uses_defaults() {
        let toml = r#"project_name = "test""#;
        let config = super::from_str(toml).expect("minimal TOML should parse");

        assert_eq!(config.project_name, "test");
        assert_eq!(config.specs_dir, "specs/");
        assert!(config.gates.is_none());
    }

    #[test]
    fn from_str_gates_section_uses_defaults() {
        let toml = r#"
project_name = "test"

[gates]
"#;
        let config = super::from_str(toml).expect("gates with defaults should parse");

        let gates = config.gates.expect("gates should be Some");
        assert_eq!(gates.default_timeout, 300);
        assert!(gates.working_dir.is_none());
    }

    #[test]
    fn from_str_invalid_toml_syntax() {
        let toml = "this is not valid toml ===";
        let err = super::from_str(toml).unwrap_err();
        let msg = err.to_string();

        // toml crate errors include line/column info
        assert!(
            msg.contains("TOML parse error"),
            "should contain parse error info, got: {msg}"
        );
    }

    #[test]
    fn from_str_rejects_unknown_keys() {
        let toml = r#"
project_name = "test"
unknown_key = "oops"
"#;
        let err = super::from_str(toml).unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("unknown field"),
            "should mention unknown field, got: {msg}"
        );
        assert!(
            msg.contains("unknown_key"),
            "should mention the bad field name, got: {msg}"
        );
    }

    #[test]
    fn from_str_rejects_unknown_gates_keys() {
        let toml = r#"
project_name = "test"

[gates]
unknown_gate_option = true
"#;
        let err = super::from_str(toml).unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("unknown field"),
            "should reject unknown gates key, got: {msg}"
        );
    }
}
