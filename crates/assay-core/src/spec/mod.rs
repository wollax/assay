//! Spec authoring and validation.
//!
//! Handles creating, parsing, and validating specifications
//! that define what should be built and their acceptance criteria.

use std::fmt;
use std::path::Path;

use assay_types::Spec;

use crate::error::{AssayError, Result};

/// A single validation issue in a spec file.
#[derive(Debug, Clone)]
pub struct SpecError {
    /// The field path (e.g., "name", "criteria").
    pub field: String,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Result of scanning a directory for spec files.
#[derive(Debug)]
pub struct ScanResult {
    /// Successfully parsed and validated specs, sorted by filename slug.
    pub specs: Vec<(String, Spec)>,
    /// Errors from files that failed to parse or validate.
    pub errors: Vec<AssayError>,
}

/// Parse a spec from a TOML string without validation.
pub fn from_str(s: &str) -> std::result::Result<Spec, toml::de::Error> {
    todo!()
}

/// Validate a parsed spec for semantic correctness.
pub fn validate(spec: &Spec) -> std::result::Result<(), Vec<SpecError>> {
    todo!()
}

/// Load and validate a spec from a file path.
pub fn load(path: &Path) -> Result<Spec> {
    todo!()
}

/// Scan a directory for `.toml` spec files.
pub fn scan(specs_dir: &Path) -> Result<ScanResult> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::Criterion;
    use std::io::Write as _;

    // ── from_str tests ──────────────────────────────────────────────

    #[test]
    fn from_str_valid_minimal() {
        let toml = r#"
name = "test"

[[criteria]]
name = "c1"
description = "d1"
"#;
        let spec = from_str(toml).expect("valid TOML should parse");

        assert_eq!(spec.name, "test");
        assert_eq!(spec.description, "");
        assert_eq!(spec.criteria.len(), 1);
        assert_eq!(spec.criteria[0].name, "c1");
        assert_eq!(spec.criteria[0].description, "d1");
        assert_eq!(spec.criteria[0].cmd, None);
    }

    #[test]
    fn from_str_valid_with_description_and_cmd() {
        let toml = r#"
name = "test"
description = "a spec"

[[criteria]]
name = "c1"
description = "d1"
cmd = "echo hi"
"#;
        let spec = from_str(toml).expect("valid TOML should parse");

        assert_eq!(spec.name, "test");
        assert_eq!(spec.description, "a spec");
        assert_eq!(spec.criteria.len(), 1);
        assert_eq!(spec.criteria[0].cmd, Some("echo hi".to_string()));
    }

    #[test]
    fn from_str_valid_multiple_criteria() {
        let toml = r#"
name = "test"

[[criteria]]
name = "c1"
description = "first"

[[criteria]]
name = "c2"
description = "second"
cmd = "cargo test"
"#;
        let spec = from_str(toml).expect("valid TOML should parse");

        assert_eq!(spec.criteria.len(), 2);
        assert_eq!(spec.criteria[0].name, "c1");
        assert_eq!(spec.criteria[1].name, "c2");
        assert_eq!(spec.criteria[1].cmd, Some("cargo test".to_string()));
    }

    #[test]
    fn from_str_description_omitted_defaults_to_empty() {
        let toml = r#"
name = "test"

[[criteria]]
name = "c1"
description = "d1"
"#;
        let spec = from_str(toml).expect("valid TOML should parse");
        assert_eq!(spec.description, "");
    }

    #[test]
    fn from_str_invalid_toml_syntax() {
        let err = from_str("this is not valid toml ===").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("TOML parse error"),
            "should contain parse error info, got: {msg}"
        );
    }

    #[test]
    fn from_str_rejects_unknown_spec_field() {
        let toml = r#"
name = "test"
unknown_key = "oops"

[[criteria]]
name = "c1"
description = "d1"
"#;
        let err = from_str(toml).unwrap_err();
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
    fn from_str_rejects_unknown_criterion_field() {
        let toml = r#"
name = "test"

[[criteria]]
name = "c1"
description = "d1"
unknown_crit_key = true
"#;
        let err = from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown criterion key, got: {msg}"
        );
    }

    // ── validate tests ──────────────────────────────────────────────

    fn valid_spec() -> Spec {
        Spec {
            name: "test".to_string(),
            description: String::new(),
            criteria: vec![Criterion {
                name: "c1".to_string(),
                description: "d1".to_string(),
                cmd: None,
            }],
        }
    }

    #[test]
    fn validate_valid_spec_returns_ok() {
        assert!(validate(&valid_spec()).is_ok());
    }

    #[test]
    fn validate_empty_name() {
        let mut spec = valid_spec();
        spec.name = String::new();

        let errors = validate(&spec).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
        assert!(
            errors[0].message.contains("must not be empty"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_whitespace_only_name() {
        let mut spec = valid_spec();
        spec.name = "   \t  ".to_string();

        let errors = validate(&spec).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[test]
    fn validate_zero_criteria() {
        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            criteria: vec![],
        };

        let errors = validate(&spec).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "criteria");
        assert!(
            errors[0].message.contains("at least one"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_duplicate_criterion_names() {
        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            criteria: vec![
                Criterion {
                    name: "dup".to_string(),
                    description: "first".to_string(),
                    cmd: None,
                },
                Criterion {
                    name: "dup".to_string(),
                    description: "second".to_string(),
                    cmd: None,
                },
            ],
        };

        let errors = validate(&spec).unwrap_err();
        let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
        assert!(
            msgs.iter().any(|m| m.contains("dup")),
            "should identify the duplicate name, got: {msgs:?}"
        );
    }

    #[test]
    fn validate_empty_criterion_name() {
        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            criteria: vec![Criterion {
                name: String::new(),
                description: "d1".to_string(),
                cmd: None,
            }],
        };

        let errors = validate(&spec).unwrap_err();
        assert!(errors.iter().any(|e| e.field.contains("criteria")));
    }

    #[test]
    fn validate_collects_all_errors_at_once() {
        let spec = Spec {
            name: "   ".to_string(),
            description: String::new(),
            criteria: vec![],
        };

        let errors = validate(&spec).unwrap_err();
        assert_eq!(
            errors.len(),
            2,
            "should collect both empty name and no criteria, got: {errors:?}"
        );
    }

    // ── load tests ──────────────────────────────────────────────────

    fn write_spec_file(dir: &std::path::Path, filename: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(filename);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn load_valid_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec_file(
            dir.path(),
            "test.toml",
            r#"
name = "loaded"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );

        let spec = load(&path).expect("valid spec should load");
        assert_eq!(spec.name, "loaded");
        assert_eq!(spec.criteria.len(), 1);
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
    fn load_invalid_toml_returns_spec_parse() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec_file(dir.path(), "bad.toml", "not valid toml ===");

        let err = load(&path).unwrap_err();
        match &err {
            AssayError::SpecParse { path: p, message } => {
                assert!(
                    p.ends_with("bad.toml"),
                    "path should end with bad.toml, got: {p:?}"
                );
                assert!(
                    message.contains("TOML parse error"),
                    "message should contain parse error, got: {message}"
                );
            }
            other => panic!("expected SpecParse, got: {other:?}"),
        }
    }

    #[test]
    fn load_valid_toml_invalid_semantics_returns_spec_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_spec_file(
            dir.path(),
            "empty-name.toml",
            r#"
name = ""

[[criteria]]
name = "c1"
description = "d1"
"#,
        );

        let err = load(&path).unwrap_err();
        match &err {
            AssayError::SpecValidation { path: p, errors } => {
                assert!(
                    p.ends_with("empty-name.toml"),
                    "path should end with empty-name.toml, got: {p:?}"
                );
                assert!(
                    !errors.is_empty(),
                    "should have at least one validation error"
                );
            }
            other => panic!("expected SpecValidation, got: {other:?}"),
        }
    }

    // ── scan tests ──────────────────────────────────────────────────

    fn write_spec_in(dir: &std::path::Path, filename: &str, content: &str) {
        let path = dir.join(filename);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn scan_valid_specs() {
        let dir = tempfile::tempdir().unwrap();
        write_spec_in(
            dir.path(),
            "alpha.toml",
            r#"
name = "alpha"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );
        write_spec_in(
            dir.path(),
            "beta.toml",
            r#"
name = "beta"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );

        let result = scan(dir.path()).expect("scan should succeed");
        assert_eq!(result.specs.len(), 2);
        assert!(result.errors.is_empty());
        // Sorted by filename
        assert_eq!(result.specs[0].0, "alpha");
        assert_eq!(result.specs[1].0, "beta");
        assert_eq!(result.specs[0].1.name, "alpha");
        assert_eq!(result.specs[1].1.name, "beta");
    }

    #[test]
    fn scan_mixed_valid_and_invalid() {
        let dir = tempfile::tempdir().unwrap();
        write_spec_in(
            dir.path(),
            "good.toml",
            r#"
name = "good"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );
        write_spec_in(dir.path(), "bad.toml", "not valid toml ===");

        let result = scan(dir.path()).expect("scan should succeed even with errors");
        assert_eq!(result.specs.len(), 1);
        assert_eq!(result.specs[0].0, "good");
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn scan_ignores_non_toml_files() {
        let dir = tempfile::tempdir().unwrap();
        write_spec_in(
            dir.path(),
            "spec.toml",
            r#"
name = "spec"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );
        write_spec_in(dir.path(), "readme.md", "# Not a spec");
        write_spec_in(dir.path(), "notes.txt", "just notes");

        let result = scan(dir.path()).expect("scan should succeed");
        assert_eq!(result.specs.len(), 1);
        assert_eq!(result.specs[0].0, "spec");
        assert!(result.errors.is_empty());
    }

    #[test]
    fn scan_sorted_by_filename() {
        let dir = tempfile::tempdir().unwrap();
        // Write in reverse alphabetical order
        write_spec_in(
            dir.path(),
            "zeta.toml",
            r#"
name = "zeta"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );
        write_spec_in(
            dir.path(),
            "alpha.toml",
            r#"
name = "alpha"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );

        let result = scan(dir.path()).expect("scan should succeed");
        assert_eq!(result.specs[0].0, "alpha");
        assert_eq!(result.specs[1].0, "zeta");
    }

    #[test]
    fn scan_detects_duplicate_spec_names() {
        let dir = tempfile::tempdir().unwrap();
        write_spec_in(
            dir.path(),
            "first.toml",
            r#"
name = "same-name"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );
        write_spec_in(
            dir.path(),
            "second.toml",
            r#"
name = "same-name"

[[criteria]]
name = "c1"
description = "d1"
"#,
        );

        let result = scan(dir.path()).expect("scan should succeed");
        // Duplicate name is an error; at least one file should be in errors
        assert!(
            !result.errors.is_empty(),
            "duplicate spec names should produce errors"
        );
    }

    #[test]
    fn scan_nonexistent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nonexistent = dir.path().join("does-not-exist");

        let err = scan(&nonexistent).unwrap_err();
        assert!(
            matches!(err, AssayError::SpecScan { .. }),
            "expected SpecScan error, got: {err:?}"
        );
    }
}
