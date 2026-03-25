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

    // Dependency-aware structural checks — only when at least one session uses depends_on.
    let has_deps = manifest.sessions.iter().any(|s| !s.depends_on.is_empty());

    if has_deps {
        // Build effective name index for reference resolution.
        let effective_names: Vec<String> = manifest
            .sessions
            .iter()
            .map(|s| s.name.as_deref().unwrap_or(&s.spec).to_string())
            .collect();

        // Check for duplicate effective names.
        let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        let mut has_duplicates = false;
        for (i, name) in effective_names.iter().enumerate() {
            if let Some(&prev_idx) = seen.get(name.as_str()) {
                has_duplicates = true;
                errors.push(ManifestError {
                    field: format!("sessions[{i}]"),
                    message: format!(
                        "duplicate effective name '{}' (conflicts with sessions[{}])",
                        name, prev_idx
                    ),
                });
            } else {
                seen.insert(name, i);
            }
        }

        // Only check references if names are unique (otherwise resolution is ambiguous).
        if !has_duplicates {
            let name_set: std::collections::HashSet<&str> =
                effective_names.iter().map(|s| s.as_str()).collect();

            for (i, session) in manifest.sessions.iter().enumerate() {
                let self_name = &effective_names[i];
                for (j, dep) in session.depends_on.iter().enumerate() {
                    if dep == self_name {
                        errors.push(ManifestError {
                            field: format!("sessions[{i}].depends_on[{j}]"),
                            message: format!("session '{}' depends on itself", self_name),
                        });
                    } else if !name_set.contains(dep.as_str()) {
                        errors.push(ManifestError {
                            field: format!("sessions[{i}].depends_on[{j}]"),
                            message: format!(
                                "session '{}' depends on unknown session '{}'",
                                self_name, dep
                            ),
                        });
                    }
                }
            }
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
#[allow(clippy::needless_update)]
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
        let manifest = RunManifest {
            sessions: vec![],
            ..Default::default()
        };
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
                depends_on: vec![],
                file_scope: vec![],
                shared_files: vec![],
            }],
            ..Default::default()
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
                    depends_on: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                },
                assay_types::ManifestSession {
                    spec: "   ".into(),
                    name: None,
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    depends_on: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                },
            ],
            ..Default::default()
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

    // ── dependency validation tests ─────────────────────────────────

    fn session(
        spec: &str,
        name: Option<&str>,
        depends_on: Vec<&str>,
    ) -> assay_types::ManifestSession {
        assay_types::ManifestSession {
            spec: spec.into(),
            name: name.map(|n| n.into()),
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: depends_on.into_iter().map(|s| s.into()).collect(),
        }
    }

    #[test]
    fn validate_deps_unknown_reference_rejected() {
        let manifest = RunManifest {
            sessions: vec![
                session("a", None, vec![]),
                session("b", None, vec!["nonexistent"]),
            ],
            ..Default::default()
        };
        let errors = validate(&manifest).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "sessions[1].depends_on[0]");
        assert!(
            errors[0].message.contains("unknown session 'nonexistent'"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_deps_self_dependency_rejected() {
        let manifest = RunManifest {
            sessions: vec![session("a", None, vec!["a"])],
            ..Default::default()
        };
        let errors = validate(&manifest).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "sessions[0].depends_on[0]");
        assert!(
            errors[0].message.contains("depends on itself"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_deps_duplicate_names_rejected() {
        let manifest = RunManifest {
            sessions: vec![session("a", None, vec![]), session("a", None, vec!["a"])],
            ..Default::default()
        };
        let errors = validate(&manifest).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("duplicate effective name")),
            "expected duplicate name error, got: {errors:?}"
        );
    }

    #[test]
    fn validate_deps_valid_references_accepted() {
        let manifest = RunManifest {
            sessions: vec![
                session("a", None, vec![]),
                session("b", None, vec!["a"]),
                session("c", Some("custom"), vec!["a", "b"]),
            ],
            ..Default::default()
        };
        assert!(validate(&manifest).is_ok());
    }

    #[test]
    fn validate_deps_effective_name_uses_name_over_spec() {
        // Session has name="custom", spec="x". depends_on should reference "custom" not "x".
        let manifest = RunManifest {
            sessions: vec![
                session("x", Some("custom"), vec![]),
                session("b", None, vec!["x"]), // "x" is not the effective name
            ],
            ..Default::default()
        };
        let errors = validate(&manifest).unwrap_err();
        assert!(
            errors[0].message.contains("unknown session 'x'"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_no_deps_allows_duplicate_specs() {
        // Without depends_on, duplicate specs are fine (backward compat).
        let manifest = RunManifest {
            sessions: vec![session("a", None, vec![]), session("a", None, vec![])],
            ..Default::default()
        };
        assert!(validate(&manifest).is_ok());
    }

    #[test]
    fn validate_deps_collects_multiple_errors() {
        let manifest = RunManifest {
            sessions: vec![
                session("a", None, vec!["a"]),           // self-dep
                session("b", None, vec!["nonexistent"]), // unknown ref
            ],
            ..Default::default()
        };
        let errors = validate(&manifest).unwrap_err();
        assert_eq!(
            errors.len(),
            2,
            "should collect both errors, got: {errors:?}"
        );
    }
}
