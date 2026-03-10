//! Spec parsing, validation, and directory scanning.
//!
//! Handles loading, parsing, and validating specifications
//! that define what should be built and their acceptance criteria.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

use assay_types::{CriterionKind, Enforcement, FeatureSpec, GateSection, GatesSpec, Spec};

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

/// A unified spec entry discovered during directory scanning.
///
/// Represents either a legacy flat `.toml` file or a directory-based spec
/// with `gates.toml` (and optional `spec.toml`).
#[derive(Debug)]
pub enum SpecEntry {
    /// A legacy flat `.toml` spec file.
    Legacy { slug: String, spec: Spec },
    /// A directory-based spec with `gates.toml` and optional `spec.toml`.
    Directory {
        slug: String,
        gates: GatesSpec,
        /// Path to `spec.toml` if it exists (not loaded eagerly).
        spec_path: Option<PathBuf>,
    },
}

impl SpecEntry {
    /// The slug (directory name or filename stem) for this entry.
    pub fn slug(&self) -> &str {
        match self {
            Self::Legacy { slug, .. } | Self::Directory { slug, .. } => slug,
        }
    }

    /// The display name from the loaded spec/gates.
    pub fn name(&self) -> &str {
        match self {
            Self::Legacy { spec, .. } => &spec.name,
            Self::Directory { gates, .. } => &gates.name,
        }
    }
}

/// Result of scanning a directory for spec files.
#[derive(Debug)]
pub struct ScanResult {
    /// All discovered spec entries (legacy + directory), sorted by slug.
    pub entries: Vec<SpecEntry>,
    /// Successfully parsed and validated legacy specs, sorted by filename.
    /// Each entry is `(slug, spec)` where slug is the filename without extension.
    /// Populated from `entries` for backward compatibility.
    pub specs: Vec<(String, Spec)>,
    /// Errors from files that failed to parse, validate, or read.
    pub errors: Vec<AssayError>,
}

/// Parse a spec from a TOML string without validation.
///
/// Returns the raw `toml::de::Error` on failure, preserving line/column
/// information. For file-based loading with automatic validation, see
/// [`load()`] which reads from disk and calls [`validate()`].
pub fn from_str(s: &str) -> std::result::Result<Spec, toml::de::Error> {
    toml::from_str(s)
}

/// Validate a parsed spec for semantic correctness.
///
/// Collects **all** validation errors at once so the user can fix
/// everything in a single pass. Returns `Ok(())` when valid,
/// `Err(errors)` with every issue found otherwise.
pub fn validate(spec: &Spec) -> std::result::Result<(), Vec<SpecError>> {
    let mut errors = Vec::new();

    if spec.name.trim().is_empty() {
        errors.push(SpecError {
            field: "name".into(),
            message: "required, must not be empty".into(),
        });
    }

    if spec.criteria.is_empty() {
        errors.push(SpecError {
            field: "criteria".into(),
            message: "must have at least one criterion".into(),
        });
    } else {
        validate_criteria(&spec.criteria, spec.gate.as_ref(), &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate per-criterion rules shared by both legacy specs and gate specs.
///
/// Checks each criterion for:
/// - Non-empty, unique names
/// - `cmd` / `path` mutual exclusion
/// - `AgentReport` incompatibility with `cmd` / `path`
/// - At least one executable criterion with `enforcement = required`
fn validate_criteria(
    criteria: &[assay_types::Criterion],
    gate: Option<&GateSection>,
    errors: &mut Vec<SpecError>,
) {
    let mut seen = HashSet::new();
    for (i, criterion) in criteria.iter().enumerate() {
        if criterion.name.trim().is_empty() {
            errors.push(SpecError {
                field: format!("criteria[{i}].name"),
                message: "required, must not be empty".into(),
            });
        } else if !seen.insert(&criterion.name) {
            errors.push(SpecError {
                field: format!("criteria[{i}].name"),
                message: format!("duplicate criterion name `{}`", criterion.name),
            });
        }

        if criterion.cmd.is_some() && criterion.path.is_some() {
            errors.push(SpecError {
                field: format!("criteria[{i}]"),
                message: format!(
                    "criterion `{}` has both `cmd` and `path`; `cmd` takes precedence, `path` is ignored",
                    criterion.name
                ),
            });
        }

        // AgentReport criteria must not have cmd or path
        if criterion.kind == Some(CriterionKind::AgentReport) {
            if criterion.cmd.is_some() {
                errors.push(SpecError {
                    field: format!("criteria[{i}]"),
                    message: format!(
                        "criterion `{}` has kind=AgentReport with `cmd`; agent criteria cannot have a command",
                        criterion.name
                    ),
                });
            }
            if criterion.path.is_some() {
                errors.push(SpecError {
                    field: format!("criteria[{i}]"),
                    message: format!(
                        "criterion `{}` has kind=AgentReport with `path`; agent criteria cannot have a path check",
                        criterion.name
                    ),
                });
            }
        }
    }

    // Verify at least one executable criterion is required.
    // AgentReport criteria count as "executable" (evaluated through sessions).
    let is_executable = |c: &assay_types::Criterion| {
        c.cmd.is_some() || c.path.is_some() || c.kind == Some(CriterionKind::AgentReport)
    };
    let has_executable = criteria.iter().any(&is_executable);
    let has_required_executable = criteria.iter().any(|c| {
        let enforcement = crate::gate::resolve_enforcement(c.enforcement, gate);
        is_executable(c) && enforcement == Enforcement::Required
    });

    if !has_executable {
        errors.push(SpecError {
            field: "criteria".into(),
            message: "at least one criterion must have a `cmd` or `path` field".into(),
        });
    } else if !has_required_executable {
        errors.push(SpecError {
            field: "criteria".into(),
            message: "at least one executable criterion must have enforcement = \"required\"; a gate with only advisory criteria would always pass".into(),
        });
    }
}

/// Load and validate a spec from a file path.
///
/// Reads the file, parses it as TOML, and validates the result. Wraps
/// parse errors in [`AssayError::SpecParse`] (with file path) and
/// validation errors in [`AssayError::SpecValidation`].
pub fn load(path: &Path) -> Result<Spec> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading spec".into(),
        path: path.to_path_buf(),
        source,
    })?;

    let spec: Spec = toml::from_str(&content).map_err(|e| AssayError::SpecParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    if let Err(errors) = validate(&spec) {
        return Err(AssayError::SpecValidation {
            path: path.to_path_buf(),
            errors,
        });
    }

    Ok(spec)
}

/// Load and validate a gates spec from a `gates.toml` file path.
pub fn load_gates(path: &Path) -> Result<GatesSpec> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading gates spec".into(),
        path: path.to_path_buf(),
        source,
    })?;

    let gates: GatesSpec = toml::from_str(&content).map_err(|e| AssayError::GatesSpecParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    if let Err(errors) = validate_gates_spec(&gates) {
        return Err(AssayError::GatesSpecValidation {
            path: path.to_path_buf(),
            errors,
        });
    }

    Ok(gates)
}

/// Load and validate a feature spec from a `spec.toml` file path.
pub fn load_feature_spec(path: &Path) -> Result<FeatureSpec> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading feature spec".into(),
        path: path.to_path_buf(),
        source,
    })?;

    let spec: FeatureSpec = toml::from_str(&content).map_err(|e| AssayError::FeatureSpecParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    if let Err(errors) = validate_feature_spec(&spec) {
        return Err(AssayError::FeatureSpecValidation {
            path: path.to_path_buf(),
            errors,
        });
    }

    Ok(spec)
}

/// Unified spec lookup: tries directory first (gates.toml), then flat file.
///
/// Returns `SpecNotFound` if neither exists.
pub fn load_spec_entry(slug: &str, specs_dir: &Path) -> Result<SpecEntry> {
    let dir_path = specs_dir.join(slug);
    let gates_path = dir_path.join("gates.toml");

    // Try directory-based spec first
    if gates_path.is_file() {
        let gates = load_gates(&gates_path)?;
        let spec_path = dir_path.join("spec.toml");
        let spec_path = if spec_path.is_file() {
            Some(spec_path)
        } else {
            None
        };
        return Ok(SpecEntry::Directory {
            slug: slug.to_string(),
            gates,
            spec_path,
        });
    }

    // Try legacy flat file
    let flat_path = specs_dir.join(format!("{slug}.toml"));
    if flat_path.is_file() {
        let spec = load(&flat_path)?;
        return Ok(SpecEntry::Legacy {
            slug: slug.to_string(),
            spec,
        });
    }

    Err(AssayError::SpecNotFound {
        name: slug.to_string(),
        specs_dir: specs_dir.to_path_buf(),
    })
}

/// Validate a feature spec for semantic correctness.
///
/// Checks: non-empty name, valid REQ-ID format, no duplicate requirement IDs.
pub fn validate_feature_spec(spec: &FeatureSpec) -> std::result::Result<(), Vec<SpecError>> {
    let mut errors = Vec::new();

    if spec.name.trim().is_empty() {
        errors.push(SpecError {
            field: "name".into(),
            message: "required, must not be empty".into(),
        });
    }

    // Validate requirement IDs: REQ-[AREA]-[NNN] format
    let mut seen_ids = HashSet::new();
    for (i, req) in spec.requirements.iter().enumerate() {
        if req.id.trim().is_empty() {
            errors.push(SpecError {
                field: format!("requirements[{i}].id"),
                message: "required, must not be empty".into(),
            });
        } else if !is_valid_req_id(&req.id) {
            errors.push(SpecError {
                field: format!("requirements[{i}].id"),
                message: format!(
                    "invalid requirement ID `{}`; expected REQ-<AREA>-<NNN> format",
                    req.id
                ),
            });
        } else if !seen_ids.insert(&req.id) {
            errors.push(SpecError {
                field: format!("requirements[{i}].id"),
                message: format!("duplicate requirement ID `{}`", req.id),
            });
        }

        if req.title.trim().is_empty() {
            errors.push(SpecError {
                field: format!("requirements[{i}].title"),
                message: "required, must not be empty".into(),
            });
        }

        if req.statement.trim().is_empty() {
            errors.push(SpecError {
                field: format!("requirements[{i}].statement"),
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

/// Validate a gates spec for semantic correctness.
///
/// Same rules as legacy spec validation: non-empty name, at least one criterion,
/// no duplicate criterion names.
pub fn validate_gates_spec(spec: &GatesSpec) -> std::result::Result<(), Vec<SpecError>> {
    let mut errors = Vec::new();

    if spec.name.trim().is_empty() {
        errors.push(SpecError {
            field: "name".into(),
            message: "required, must not be empty".into(),
        });
    }

    if spec.criteria.is_empty() {
        errors.push(SpecError {
            field: "criteria".into(),
            message: "must have at least one criterion".into(),
        });
    } else {
        validate_criteria(&spec.criteria, spec.gate.as_ref(), &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a requirement ID follows the `REQ-AREA-NNN` pattern.
///
/// Hand-written parser (no regex dependency):
/// - Must start with `REQ-`
/// - Followed by an uppercase-alpha area segment (1+ chars)
/// - Then `-`
/// - Then digits (1+ chars)
fn is_valid_req_id(id: &str) -> bool {
    let rest = match id.strip_prefix("REQ-") {
        Some(r) => r,
        None => return false,
    };

    // Find the last `-` that separates AREA from NNN
    let last_dash = match rest.rfind('-') {
        Some(pos) => pos,
        None => return false,
    };

    let area = &rest[..last_dash];
    let number = &rest[last_dash + 1..];

    // Area: non-empty, all uppercase ASCII letters or hyphens (for multi-word areas)
    if area.is_empty() {
        return false;
    }
    if !area.chars().all(|c| c.is_ascii_uppercase() || c == '-') {
        return false;
    }
    // Area must not start or end with hyphen
    if area.starts_with('-') || area.ends_with('-') {
        return false;
    }

    // Number: non-empty, all digits
    if number.is_empty() {
        return false;
    }
    number.chars().all(|c| c.is_ascii_digit())
}

/// Scan a directory for spec files (both flat `.toml` and directory-based).
///
/// Performs a flat (non-recursive) scan. Detects:
/// - `.toml` files → loaded as legacy `Spec`
/// - Subdirectories containing `gates.toml` → loaded as `GatesSpec`
///
/// Results are sorted by slug. After loading all valid specs,
/// duplicate names across entries are detected and reported as errors.
/// The legacy `specs` field is populated from `Legacy` entries for backward compatibility.
pub fn scan(specs_dir: &Path) -> Result<ScanResult> {
    let dir_entries = std::fs::read_dir(specs_dir).map_err(|source| AssayError::SpecScan {
        path: specs_dir.to_path_buf(),
        source,
    })?;

    // Collect all directory entries
    let mut scan_errors = Vec::new();
    let mut fs_entries: Vec<_> = dir_entries
        .filter_map(|entry| match entry {
            Ok(e) => Some(e),
            Err(source) => {
                scan_errors.push(AssayError::Io {
                    operation: "reading directory entry".into(),
                    path: specs_dir.to_path_buf(),
                    source,
                });
                None
            }
        })
        .collect();
    fs_entries.sort_by_key(|e| e.path());

    let mut entries = Vec::new();
    let mut errors = scan_errors;

    for fs_entry in &fs_entries {
        let path = fs_entry.path();

        // Directory-based spec: subdirectory with gates.toml
        if path.is_dir() {
            let gates_path = path.join("gates.toml");
            if gates_path.is_file() {
                let slug = match path.file_name().and_then(|s| s.to_str()) {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => continue,
                };
                match load_gates(&gates_path) {
                    Ok(gates) => {
                        let spec_path = path.join("spec.toml");
                        let spec_path = if spec_path.is_file() {
                            Some(spec_path)
                        } else {
                            None
                        };
                        entries.push(SpecEntry::Directory {
                            slug,
                            gates,
                            spec_path,
                        });
                    }
                    Err(err) => errors.push(err),
                }
            }
            continue;
        }

        // Legacy flat .toml file
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load(&path) {
                Ok(spec) => {
                    let slug = match path.file_stem().and_then(|s| s.to_str()) {
                        Some(s) if !s.is_empty() => s.to_string(),
                        _ => {
                            errors.push(AssayError::Io {
                                operation: "extracting filename stem".into(),
                                path: path.clone(),
                                source: std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "file has no valid stem",
                                ),
                            });
                            continue;
                        }
                    };
                    entries.push(SpecEntry::Legacy { slug, spec });
                }
                Err(err) => errors.push(err),
            }
        }
    }

    // Detect duplicate names across all entries
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut duplicate_indices = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        if !seen_names.insert(entry.name().to_string()) {
            duplicate_indices.push(i);
        }
    }

    // Remove duplicates in reverse order and report as errors
    for i in duplicate_indices.into_iter().rev() {
        let removed = entries.remove(i);
        let path = match &removed {
            SpecEntry::Legacy { slug, .. } => specs_dir.join(format!("{slug}.toml")),
            SpecEntry::Directory { slug, .. } => specs_dir.join(slug).join("gates.toml"),
        };
        errors.push(AssayError::SpecValidation {
            path,
            errors: vec![SpecError {
                field: "name".into(),
                message: format!("duplicate spec name `{}`", removed.name()),
            }],
        });
    }

    // Build backward-compatible specs vec from Legacy entries
    let specs: Vec<(String, Spec)> = entries
        .iter()
        .filter_map(|entry| match entry {
            SpecEntry::Legacy { slug, spec } => Some((slug.clone(), spec.clone())),
            SpecEntry::Directory { .. } => None,
        })
        .collect();

    Ok(ScanResult {
        entries,
        specs,
        errors,
    })
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
            gate: None,
            criteria: vec![Criterion {
                name: "c1".to_string(),
                description: "d1".to_string(),
                cmd: Some("true".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
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
            gate: None,
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
            gate: None,
            criteria: vec![
                Criterion {
                    name: "dup".to_string(),
                    description: "first".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "dup".to_string(),
                    description: "second".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
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
            gate: None,
            criteria: vec![Criterion {
                name: String::new(),
                description: "d1".to_string(),
                cmd: None,
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
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
            gate: None,
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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
cmd = "true"
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

    // ── directory-based spec tests ──────────────────────────────────

    fn create_dir_spec(
        specs_dir: &std::path::Path,
        name: &str,
        gates_toml: &str,
        spec_toml: Option<&str>,
    ) {
        let dir = specs_dir.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("gates.toml"), gates_toml).unwrap();
        if let Some(content) = spec_toml {
            std::fs::write(dir.join("spec.toml"), content).unwrap();
        }
    }

    #[test]
    fn load_gates_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gates.toml");
        std::fs::write(
            &path,
            r#"
name = "auth-flow"

[[criteria]]
name = "compiles"
description = "Code compiles"
cmd = "cargo build"
requirements = ["REQ-FUNC-001"]
"#,
        )
        .unwrap();

        let gates = load_gates(&path).expect("valid gates spec");
        assert_eq!(gates.name, "auth-flow");
        assert_eq!(gates.criteria.len(), 1);
        assert_eq!(gates.criteria[0].requirements, vec!["REQ-FUNC-001"]);
    }

    #[test]
    fn load_gates_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gates.toml");
        std::fs::write(&path, "not valid toml ===").unwrap();

        let err = load_gates(&path).unwrap_err();
        assert!(
            matches!(err, AssayError::GatesSpecParse { .. }),
            "expected GatesSpecParse, got: {err:?}"
        );
    }

    #[test]
    fn load_gates_empty_name_fails_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gates.toml");
        std::fs::write(
            &path,
            r#"
name = ""

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
        )
        .unwrap();

        let err = load_gates(&path).unwrap_err();
        assert!(
            matches!(err, AssayError::GatesSpecValidation { .. }),
            "expected GatesSpecValidation, got: {err:?}"
        );
    }

    #[test]
    fn load_feature_spec_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spec.toml");
        std::fs::write(
            &path,
            r#"
name = "auth-flow"
status = "draft"

[[requirements]]
id = "REQ-FUNC-001"
title = "Login"
statement = "Users can log in"
"#,
        )
        .unwrap();

        let spec = load_feature_spec(&path).expect("valid feature spec");
        assert_eq!(spec.name, "auth-flow");
        assert_eq!(spec.requirements.len(), 1);
    }

    #[test]
    fn load_feature_spec_invalid_req_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spec.toml");
        std::fs::write(
            &path,
            r#"
name = "auth-flow"

[[requirements]]
id = "BAD-ID"
title = "Login"
statement = "Users can log in"
"#,
        )
        .unwrap();

        let err = load_feature_spec(&path).unwrap_err();
        assert!(
            matches!(err, AssayError::FeatureSpecValidation { .. }),
            "expected FeatureSpecValidation, got: {err:?}"
        );
    }

    #[test]
    fn load_spec_entry_finds_directory() {
        let dir = tempfile::tempdir().unwrap();
        create_dir_spec(
            dir.path(),
            "auth-flow",
            r#"
name = "auth-flow"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
            Some(
                r#"
name = "auth-flow"
status = "draft"
"#,
            ),
        );

        let entry = load_spec_entry("auth-flow", dir.path()).expect("should find directory spec");
        assert!(
            matches!(entry, SpecEntry::Directory { ref spec_path, .. } if spec_path.is_some()),
            "expected Directory with spec_path, got: {entry:?}"
        );
        assert_eq!(entry.slug(), "auth-flow");
    }

    #[test]
    fn load_spec_entry_finds_legacy() {
        let dir = tempfile::tempdir().unwrap();
        write_spec_in(
            dir.path(),
            "hello.toml",
            r#"
name = "hello"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
        );

        let entry = load_spec_entry("hello", dir.path()).expect("should find legacy spec");
        assert!(
            matches!(entry, SpecEntry::Legacy { .. }),
            "expected Legacy, got: {entry:?}"
        );
    }

    #[test]
    fn load_spec_entry_prefers_directory_over_flat() {
        let dir = tempfile::tempdir().unwrap();
        // Create both a flat file and a directory with the same slug
        write_spec_in(
            dir.path(),
            "auth.toml",
            r#"
name = "auth"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
        );
        create_dir_spec(
            dir.path(),
            "auth",
            r#"
name = "auth"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
            None,
        );

        let entry = load_spec_entry("auth", dir.path()).expect("should find spec");
        assert!(
            matches!(entry, SpecEntry::Directory { .. }),
            "directory should take priority over flat file"
        );
    }

    #[test]
    fn load_spec_entry_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_spec_entry("nonexistent", dir.path()).unwrap_err();
        assert!(
            matches!(err, AssayError::SpecNotFound { .. }),
            "expected SpecNotFound, got: {err:?}"
        );
    }

    #[test]
    fn scan_discovers_directory_specs() {
        let dir = tempfile::tempdir().unwrap();
        create_dir_spec(
            dir.path(),
            "auth-flow",
            r#"
name = "auth-flow"

[[criteria]]
name = "c1"
description = "d1"
cmd = "echo ok"
"#,
            None,
        );

        let result = scan(dir.path()).expect("scan should succeed");
        assert_eq!(result.entries.len(), 1);
        assert!(
            matches!(&result.entries[0], SpecEntry::Directory { slug, .. } if slug == "auth-flow"),
            "expected Directory entry"
        );
        // Legacy specs vec should be empty (no flat files)
        assert!(result.specs.is_empty());
    }

    #[test]
    fn scan_mixed_legacy_and_directory() {
        let dir = tempfile::tempdir().unwrap();
        write_spec_in(
            dir.path(),
            "alpha.toml",
            r#"
name = "alpha"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
        );
        create_dir_spec(
            dir.path(),
            "beta-dir",
            r#"
name = "beta-dir"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
            None,
        );

        let result = scan(dir.path()).expect("scan should succeed");
        assert_eq!(result.entries.len(), 2);
        assert!(result.errors.is_empty());

        // Legacy specs should only contain the flat file
        assert_eq!(result.specs.len(), 1);
        assert_eq!(result.specs[0].0, "alpha");
    }

    #[test]
    fn scan_detects_cross_format_duplicate_names() {
        let dir = tempfile::tempdir().unwrap();
        write_spec_in(
            dir.path(),
            "dupe.toml",
            r#"
name = "same-name"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
        );
        create_dir_spec(
            dir.path(),
            "dupe-dir",
            r#"
name = "same-name"

[[criteria]]
name = "c1"
description = "d1"
cmd = "true"
"#,
            None,
        );

        let result = scan(dir.path()).expect("scan should succeed");
        assert!(
            !result.errors.is_empty(),
            "should detect cross-format duplicate names"
        );
    }

    #[test]
    fn scan_ignores_directories_without_gates_toml() {
        let dir = tempfile::tempdir().unwrap();
        // Create a directory without gates.toml
        std::fs::create_dir_all(dir.path().join("empty-dir")).unwrap();
        std::fs::write(dir.path().join("empty-dir").join("readme.md"), "not a spec").unwrap();

        let result = scan(dir.path()).expect("scan should succeed");
        assert!(result.entries.is_empty());
        assert!(result.errors.is_empty());
    }

    // ── validate_feature_spec tests ─────────────────────────────────

    #[test]
    fn validate_feature_spec_valid() {
        let spec = FeatureSpec {
            name: "test".into(),
            status: Default::default(),
            version: String::new(),
            overview: None,
            constraints: None,
            users: vec![],
            requirements: vec![assay_types::feature_spec::Requirement {
                id: "REQ-FUNC-001".into(),
                title: "Test".into(),
                statement: "The system SHALL test".into(),
                rationale: String::new(),
                obligation: Default::default(),
                priority: Default::default(),
                verification: Default::default(),
                status: Default::default(),
                acceptance_criteria: vec![],
            }],
            quality: None,
            assumptions: vec![],
            dependencies: vec![],
            risks: vec![],
            verification: None,
        };
        assert!(validate_feature_spec(&spec).is_ok());
    }

    #[test]
    fn validate_feature_spec_empty_name() {
        let spec = FeatureSpec {
            name: "".into(),
            status: Default::default(),
            version: String::new(),
            overview: None,
            constraints: None,
            users: vec![],
            requirements: vec![],
            quality: None,
            assumptions: vec![],
            dependencies: vec![],
            risks: vec![],
            verification: None,
        };
        let errors = validate_feature_spec(&spec).unwrap_err();
        assert!(errors.iter().any(|e| e.field == "name"));
    }

    #[test]
    fn validate_feature_spec_duplicate_req_ids() {
        let req = assay_types::feature_spec::Requirement {
            id: "REQ-FUNC-001".into(),
            title: "Test".into(),
            statement: "Statement".into(),
            rationale: String::new(),
            obligation: Default::default(),
            priority: Default::default(),
            verification: Default::default(),
            status: Default::default(),
            acceptance_criteria: vec![],
        };
        let spec = FeatureSpec {
            name: "test".into(),
            status: Default::default(),
            version: String::new(),
            overview: None,
            constraints: None,
            users: vec![],
            requirements: vec![req.clone(), req],
            quality: None,
            assumptions: vec![],
            dependencies: vec![],
            risks: vec![],
            verification: None,
        };
        let errors = validate_feature_spec(&spec).unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("duplicate")));
    }

    #[test]
    fn validate_feature_spec_invalid_req_id_format() {
        let spec = FeatureSpec {
            name: "test".into(),
            status: Default::default(),
            version: String::new(),
            overview: None,
            constraints: None,
            users: vec![],
            requirements: vec![assay_types::feature_spec::Requirement {
                id: "BAD-FORMAT".into(),
                title: "Test".into(),
                statement: "Statement".into(),
                rationale: String::new(),
                obligation: Default::default(),
                priority: Default::default(),
                verification: Default::default(),
                status: Default::default(),
                acceptance_criteria: vec![],
            }],
            quality: None,
            assumptions: vec![],
            dependencies: vec![],
            risks: vec![],
            verification: None,
        };
        let errors = validate_feature_spec(&spec).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("invalid requirement ID"))
        );
    }

    // ── is_valid_req_id tests ───────────────────────────────────────

    #[test]
    fn req_id_valid_formats() {
        assert!(is_valid_req_id("REQ-FUNC-001"));
        assert!(is_valid_req_id("REQ-SEC-42"));
        assert!(is_valid_req_id("REQ-NON-FUNC-1"));
        assert!(is_valid_req_id("REQ-UI-UX-003"));
    }

    #[test]
    fn req_id_invalid_formats() {
        assert!(!is_valid_req_id(""));
        assert!(!is_valid_req_id("FUNC-001"));
        assert!(!is_valid_req_id("REQ-001"));
        assert!(!is_valid_req_id("REQ--001"));
        assert!(!is_valid_req_id("REQ-func-001")); // lowercase
        assert!(!is_valid_req_id("REQ-FUNC-"));
        assert!(!is_valid_req_id("REQ-FUNC-abc"));
    }

    // ── validate_gates_spec tests ───────────────────────────────────

    #[test]
    fn validate_gates_spec_valid() {
        let spec = GatesSpec {
            name: "test".into(),
            description: String::new(),
            gate: None,
            criteria: vec![assay_types::GateCriterion {
                name: "c1".into(),
                description: "d1".into(),
                cmd: Some("true".into()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };
        assert!(validate_gates_spec(&spec).is_ok());
    }

    #[test]
    fn validate_gates_spec_empty_criteria() {
        let spec = GatesSpec {
            name: "test".into(),
            description: String::new(),
            gate: None,
            criteria: vec![],
        };
        let errors = validate_gates_spec(&spec).unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("at least one")));
    }

    #[test]
    fn validate_gates_spec_duplicate_criterion_names() {
        let spec = GatesSpec {
            name: "test".into(),
            description: String::new(),
            gate: None,
            criteria: vec![
                assay_types::GateCriterion {
                    name: "dup".into(),
                    description: "d1".into(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                assay_types::GateCriterion {
                    name: "dup".into(),
                    description: "d2".into(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };
        let errors = validate_gates_spec(&spec).unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("dup")));
    }

    // ── enforcement validation tests ─────────────────────────────────

    #[test]
    fn validate_rejects_all_advisory_criteria() {
        use assay_types::enforcement::{Enforcement, GateSection};

        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            gate: Some(GateSection {
                enforcement: Enforcement::Advisory,
            }),
            criteria: vec![Criterion {
                name: "lint".to_string(),
                description: "run lint".to_string(),
                cmd: Some("cargo clippy".to_string()),
                path: None,
                timeout: None,
                enforcement: None, // inherits advisory from gate section
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };
        let errors = validate(&spec).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one executable criterion"))
        );
    }

    #[test]
    fn validate_accepts_required_override_on_advisory_default() {
        use assay_types::enforcement::{Enforcement, GateSection};

        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            gate: Some(GateSection {
                enforcement: Enforcement::Advisory,
            }),
            criteria: vec![Criterion {
                name: "lint".to_string(),
                description: "lint".to_string(),
                cmd: Some("cargo clippy".to_string()),
                path: None,
                timeout: None,
                enforcement: Some(Enforcement::Required),
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };
        assert!(validate(&spec).is_ok());
    }

    #[test]
    fn validate_no_gate_section_defaults_required() {
        // Existing specs without [gate] should still validate fine
        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            gate: None,
            criteria: vec![Criterion {
                name: "build".to_string(),
                description: "build".to_string(),
                cmd: Some("cargo build".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };
        assert!(validate(&spec).is_ok());
    }

    #[test]
    fn validate_gates_spec_rejects_all_advisory() {
        use assay_types::enforcement::{Enforcement, GateSection};

        let spec = GatesSpec {
            name: "test".into(),
            description: String::new(),
            gate: Some(GateSection {
                enforcement: Enforcement::Advisory,
            }),
            criteria: vec![assay_types::GateCriterion {
                name: "lint".into(),
                description: "lint".into(),
                cmd: Some("cargo clippy".into()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };
        let errors = validate_gates_spec(&spec).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one executable criterion"))
        );
    }

    #[test]
    fn validate_descriptive_required_does_not_count_as_executable() {
        use assay_types::enforcement::Enforcement;

        // A spec with one descriptive required criterion and one executable advisory criterion
        // should fail: the descriptive one doesn't count because it has no cmd/path
        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            gate: None,
            criteria: vec![
                Criterion {
                    name: "descriptive".to_string(),
                    description: "no cmd or path".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Required),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "advisory-executable".to_string(),
                    description: "has cmd but advisory".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Advisory),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };
        let errors = validate(&spec).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one executable criterion"))
        );
    }

    // ── AgentReport mutual exclusivity validation ────────────────────

    #[test]
    fn validation_rejects_agent_report_with_cmd() {
        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            gate: None,
            criteria: vec![Criterion {
                name: "agent-with-cmd".to_string(),
                description: "agent criterion with cmd".to_string(),
                cmd: Some("echo bad".to_string()),
                path: None,
                timeout: None,
                enforcement: Some(Enforcement::Required),
                kind: Some(CriterionKind::AgentReport),
                prompt: Some("Review code".to_string()),
                requirements: vec![],
            }],
        };

        let errors = validate(&spec).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("AgentReport") && e.message.contains("cmd")),
            "should reject AgentReport with cmd, got: {errors:?}"
        );
    }

    #[test]
    fn validation_rejects_agent_report_with_path() {
        let spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            gate: None,
            criteria: vec![Criterion {
                name: "agent-with-path".to_string(),
                description: "agent criterion with path".to_string(),
                cmd: None,
                path: Some("README.md".to_string()),
                timeout: None,
                enforcement: Some(Enforcement::Required),
                kind: Some(CriterionKind::AgentReport),
                prompt: Some("Check file".to_string()),
                requirements: vec![],
            }],
        };

        let errors = validate(&spec).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("AgentReport") && e.message.contains("path")),
            "should reject AgentReport with path, got: {errors:?}"
        );
    }

    #[test]
    fn scan_empty_directory_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let specs_dir = dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        let result = scan(&specs_dir).unwrap();
        assert!(
            result.entries.is_empty(),
            "empty directory should produce no entries"
        );
        assert!(
            result.errors.is_empty(),
            "empty directory should produce no errors"
        );
    }

    #[test]
    fn spec_error_display_format() {
        let err = SpecError {
            field: "criteria[0].name".to_string(),
            message: "cannot be empty".to_string(),
        };
        let display = err.to_string();
        assert!(
            display.contains("criteria[0].name"),
            "Display should contain field, got: {display}"
        );
        assert!(
            display.contains("cannot be empty"),
            "Display should contain message, got: {display}"
        );
    }
}
