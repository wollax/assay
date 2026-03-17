//! Run manifest loading and validation.
//!
//! Handles reading, parsing, and validating run manifests from TOML files.
//! Mirrors the config module pattern: `from_str()` → `validate()` → `load()`.

use std::fmt;
use std::path::Path;

use assay_types::RunManifest;

use crate::config::format_toml_error;
use crate::error::{AssayError, Result};

/// A single validation issue in a manifest file.
#[derive(Debug, Clone)]
pub struct ManifestError {
    /// The field path (e.g., "sessions", "sessions[0].spec").
    pub field: String,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Parse a run manifest from a TOML string without validation.
///
/// Returns the raw `toml::de::Error` on failure, preserving line/column
/// information. Callers that need validation should use [`load()`] instead.
pub fn from_str(s: &str) -> std::result::Result<RunManifest, toml::de::Error> {
    toml::from_str(s)
}

/// Validate a parsed manifest for semantic correctness.
///
/// Collects **all** validation errors at once so the user can fix
/// everything in a single pass. Returns `Ok(())` when valid,
/// `Err(errors)` with every issue found otherwise.
pub fn validate(manifest: &RunManifest) -> std::result::Result<(), Vec<ManifestError>> {
    let mut errors = Vec::new();

    if manifest.sessions.is_empty() {
        errors.push(ManifestError {
            field: "sessions".into(),
            message: "required, must contain at least one session".into(),
        });
    }

    for (i, session) in manifest.sessions.iter().enumerate() {
        if session.spec.trim().is_empty() {
            errors.push(ManifestError {
                field: format!("sessions[{i}].spec"),
                message: "required, must not be empty".into(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Load and validate a run manifest from a file path.
///
/// Reads the file, parses it as TOML, and validates the result. Wraps
/// parse errors in [`AssayError::ManifestParse`] (with caret-pointer display)
/// and validation errors in [`AssayError::ManifestValidation`].
pub fn load(path: &Path) -> Result<RunManifest> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading manifest".into(),
        path: path.to_path_buf(),
        source,
    })?;

    let manifest: RunManifest =
        toml::from_str(&content).map_err(|e| AssayError::ManifestParse {
            path: path.to_path_buf(),
            message: format_toml_error(&content, &e),
        })?;

    if let Err(errors) = validate(&manifest) {
        return Err(AssayError::ManifestValidation {
            path: path.to_path_buf(),
            errors,
        });
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── from_str tests ──────────────────────────────────────────────

    #[test]
    fn from_str_valid_minimal() {
        let toml = r#"
[[sessions]]
spec = "auth-flow"
"#;
        let manifest = from_str(toml).expect("valid minimal manifest should parse");
        assert_eq!(manifest.sessions.len(), 1);
        assert_eq!(manifest.sessions[0].spec, "auth-flow");
        assert!(manifest.sessions[0].name.is_none());
        assert!(manifest.sessions[0].settings.is_none());
        assert!(manifest.sessions[0].hooks.is_empty());
        assert!(manifest.sessions[0].prompt_layers.is_empty());
    }

    #[test]
    fn from_str_valid_full() {
        let toml = r#"
[[sessions]]
spec = "auth-flow"
name = "auth-with-overrides"

[sessions.settings]
model = "opus"
max_turns = 10

[[sessions.hooks]]
event = "pre-tool"
command = "echo pre"

[[sessions.prompt_layers]]
kind = "custom"
name = "security"
content = "Be extra careful with auth"
priority = 10
"#;
        let manifest = from_str(toml).expect("full manifest should parse");
        assert_eq!(manifest.sessions.len(), 1);
        let s = &manifest.sessions[0];
        assert_eq!(s.spec, "auth-flow");
        assert_eq!(s.name.as_deref(), Some("auth-with-overrides"));
        assert!(s.settings.is_some());
        assert_eq!(s.hooks.len(), 1);
        assert_eq!(s.prompt_layers.len(), 1);
    }

    #[test]
    fn from_str_multiple_sessions() {
        let toml = r#"
[[sessions]]
spec = "auth-flow"

[[sessions]]
spec = "checkout"
name = "checkout-run"
"#;
        let manifest = from_str(toml).expect("multiple sessions should parse");
        assert_eq!(manifest.sessions.len(), 2);
        assert_eq!(manifest.sessions[0].spec, "auth-flow");
        assert_eq!(manifest.sessions[1].spec, "checkout");
        assert_eq!(manifest.sessions[1].name.as_deref(), Some("checkout-run"));
    }

    #[test]
    fn from_str_round_trip() {
        let toml = r#"
[[sessions]]
spec = "auth-flow"
name = "named"

[[sessions]]
spec = "checkout"
"#;
        let parsed = from_str(toml).expect("should parse");
        let serialized = toml::to_string(&parsed).expect("should serialize");
        let reparsed = from_str(&serialized).expect("round-trip should parse");
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn from_str_rejects_unknown_fields() {
        let toml = r#"
unknown_top_level = "oops"

[[sessions]]
spec = "auth-flow"
"#;
        let err = from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should mention unknown field, got: {msg}"
        );
    }

    #[test]
    fn from_str_rejects_unknown_session_fields() {
        let toml = r#"
[[sessions]]
spec = "auth-flow"
unknown_session_key = true
"#;
        let err = from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown session key, got: {msg}"
        );
    }

    // ── validate tests ──────────────────────────────────────────────

    #[test]
    fn validate_empty_sessions_rejected() {
        let manifest = RunManifest { sessions: vec![] };
        let errors = validate(&manifest).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "sessions");
        assert!(
            errors[0].message.contains("at least one"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_empty_spec_rejected() {
        let manifest = RunManifest {
            sessions: vec![assay_types::ManifestSession {
                spec: "".into(),
                name: None,
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
            }],
        };
        let errors = validate(&manifest).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "sessions[0].spec");
        assert!(
            errors[0].message.contains("must not be empty"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_collects_all_errors() {
        let manifest = RunManifest {
            sessions: vec![
                assay_types::ManifestSession {
                    spec: "".into(),
                    name: None,
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                },
                assay_types::ManifestSession {
                    spec: "   ".into(),
                    name: None,
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                },
            ],
        };
        let errors = validate(&manifest).unwrap_err();
        assert_eq!(
            errors.len(),
            2,
            "should collect errors for both sessions, got: {errors:?}"
        );
        assert_eq!(errors[0].field, "sessions[0].spec");
        assert_eq!(errors[1].field, "sessions[1].spec");
    }

    // ── load tests ──────────────────────────────────────────────────

    #[test]
    fn load_valid_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.toml");
        std::fs::write(
            &path,
            r#"
[[sessions]]
spec = "auth-flow"
"#,
        )
        .unwrap();

        let manifest = load(&path).expect("valid manifest should load");
        assert_eq!(manifest.sessions.len(), 1);
        assert_eq!(manifest.sessions[0].spec, "auth-flow");
    }

    #[test]
    fn load_missing_file_returns_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");

        let err = load(&path).unwrap_err();
        assert!(
            matches!(err, AssayError::Io { .. }),
            "expected Io error, got: {err:?}"
        );
    }

    #[test]
    fn load_invalid_toml_returns_manifest_parse() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "not valid toml ===").unwrap();

        let err = load(&path).unwrap_err();
        match &err {
            AssayError::ManifestParse { path: p, message } => {
                assert!(
                    p.ends_with("bad.toml"),
                    "path should end with bad.toml, got: {p:?}"
                );
                assert!(
                    message.contains("line"),
                    "message should contain line info from format_toml_error, got: {message}"
                );
                assert!(
                    message.contains("^"),
                    "message should contain caret pointer, got: {message}"
                );
            }
            other => panic!("expected ManifestParse, got: {other:?}"),
        }
    }

    #[test]
    fn load_valid_toml_invalid_semantics_returns_manifest_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty-sessions.toml");
        std::fs::write(&path, "sessions = []\n").unwrap();

        let err = load(&path).unwrap_err();
        match &err {
            AssayError::ManifestValidation { path: p, errors } => {
                assert!(
                    p.ends_with("empty-sessions.toml"),
                    "path should end with empty-sessions.toml, got: {p:?}"
                );
                assert!(
                    !errors.is_empty(),
                    "should have at least one validation error"
                );
            }
            other => panic!("expected ManifestValidation, got: {other:?}"),
        }
    }
}
