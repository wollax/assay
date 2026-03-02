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

/// Validate a parsed config for semantic correctness.
///
/// Collects **all** validation errors at once so the user can fix
/// everything in a single pass. Returns `Ok(())` when valid,
/// `Err(errors)` with every issue found otherwise.
pub fn validate(config: &Config) -> std::result::Result<(), Vec<ConfigError>> {
    let mut errors = Vec::new();

    if config.project_name.trim().is_empty() {
        errors.push(ConfigError {
            field: "project_name".into(),
            message: "required, must not be empty".into(),
        });
    }

    if config.specs_dir.trim().is_empty() {
        errors.push(ConfigError {
            field: "specs_dir".into(),
            message: "required, must not be empty".into(),
        });
    }

    if let Some(gates) = &config.gates {
        if gates.default_timeout == 0 {
            errors.push(ConfigError {
                field: "[gates].default_timeout".into(),
                message: "must be a positive integer".into(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
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

    // ── validate tests ──────────────────────────────────────────────

    fn valid_config() -> assay_types::Config {
        super::from_str(r#"project_name = "test""#).unwrap()
    }

    #[test]
    fn validate_valid_config_returns_ok() {
        assert!(super::validate(&valid_config()).is_ok());
    }

    #[test]
    fn validate_valid_config_with_gates_returns_ok() {
        let config = super::from_str(
            r#"
project_name = "test"
[gates]
default_timeout = 600
"#,
        )
        .unwrap();

        assert!(super::validate(&config).is_ok());
    }

    #[test]
    fn validate_empty_project_name() {
        let mut config = valid_config();
        config.project_name = String::new();

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "project_name");
        assert!(
            errors[0].message.contains("must not be empty"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_whitespace_only_project_name() {
        let mut config = valid_config();
        config.project_name = "   \t  ".to_string();

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "project_name");
    }

    #[test]
    fn validate_empty_specs_dir() {
        let mut config = valid_config();
        config.specs_dir = String::new();

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "specs_dir");
        assert!(
            errors[0].message.contains("must not be empty"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_zero_default_timeout() {
        let mut config = valid_config();
        config.gates = Some(assay_types::GatesConfig {
            default_timeout: 0,
            working_dir: None,
        });

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "[gates].default_timeout");
        assert!(
            errors[0].message.contains("positive"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_collects_all_errors_at_once() {
        let config = assay_types::Config {
            project_name: String::new(),
            specs_dir: String::new(),
            gates: Some(assay_types::GatesConfig {
                default_timeout: 0,
                working_dir: None,
            }),
        };

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(
            errors.len(),
            3,
            "should collect all 3 errors, got: {errors:?}"
        );

        let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"project_name"));
        assert!(fields.contains(&"specs_dir"));
        assert!(fields.contains(&"[gates].default_timeout"));
    }
}
