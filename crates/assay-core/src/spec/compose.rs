//! Gate composition: slug validation, criteria library I/O, and resolution.

use std::path::{Path, PathBuf};

use assay_types::CriteriaLibrary;

use crate::error::{AssayError, Result};

/// Validate a criteria library or gate slug.
///
/// A valid slug:
/// - Is non-empty
/// - Is at most 64 characters long
/// - Consists only of ASCII lowercase letters (`a-z`), digits (`0-9`),
///   hyphens (`-`), and underscores (`_`)
/// - The first character must be an ASCII lowercase letter or digit (`[a-z0-9]`)
///
/// Returns `Ok(())` if valid, or `Err(AssayError::InvalidSlug)` describing
/// the specific violation.
pub fn validate_slug(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(AssayError::InvalidSlug {
            slug: value.to_string(),
            reason: "slug must not be empty".to_string(),
        });
    }

    if value.len() > 64 {
        return Err(AssayError::InvalidSlug {
            slug: value.to_string(),
            reason: format!("slug must be at most 64 characters, got {}", value.len()),
        });
    }

    let first = value.chars().next().expect("non-empty checked above");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(AssayError::InvalidSlug {
            slug: value.to_string(),
            reason: "first character must be an ASCII lowercase letter or digit".to_string(),
        });
    }

    for ch in value.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' && ch != '_' {
            return Err(AssayError::InvalidSlug {
                slug: value.to_string(),
                reason: format!(
                    "invalid character '{ch}': only lowercase letters, digits, hyphens, and underscores are allowed"
                ),
            });
        }
    }

    Ok(())
}

/// Load a criteria library from a TOML file.
///
/// Mirrors the `load_gates` pattern: reads the file, deserialises via toml with
/// `format_toml_error` for rich parse diagnostics. No additional semantic
/// validation step — `CriteriaLibrary`'s `deny_unknown_fields` handles schema
/// enforcement at parse time.
pub fn load_library(path: &Path) -> Result<CriteriaLibrary> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading criteria library".into(),
        path: path.to_path_buf(),
        source,
    })?;

    let lib: CriteriaLibrary = toml::from_str(&content).map_err(|e| AssayError::LibraryParse {
        path: path.to_path_buf(),
        message: crate::config::format_toml_error(&content, &e),
    })?;

    Ok(lib)
}

/// Save a criteria library to `.assay/criteria/<slug>.toml` atomically.
///
/// Validates the library's `name` field as a slug before any I/O.
/// Uses `NamedTempFile` → `write_all` → `sync_all` → `persist` for atomicity.
///
/// Returns the path of the written file on success.
pub fn save_library(assay_dir: &Path, lib: &CriteriaLibrary) -> Result<PathBuf> {
    validate_slug(&lib.name)?;

    let criteria_dir = assay_dir.join("criteria");
    std::fs::create_dir_all(&criteria_dir).map_err(|source| AssayError::Io {
        operation: "creating criteria directory".into(),
        path: criteria_dir.clone(),
        source,
    })?;

    let toml_str = toml::to_string_pretty(lib).map_err(|e| AssayError::LibraryParse {
        path: criteria_dir.join(format!("{}.toml", lib.name)),
        message: e.to_string(),
    })?;

    let final_path = criteria_dir.join(format!("{}.toml", lib.name));

    use std::io::Write as _;
    use tempfile::NamedTempFile;
    let mut tmpfile = NamedTempFile::new_in(&criteria_dir).map_err(|source| AssayError::Io {
        operation: "creating temp file for criteria library".into(),
        path: criteria_dir.clone(),
        source,
    })?;
    tmpfile
        .write_all(toml_str.as_bytes())
        .map_err(|source| AssayError::Io {
            operation: "writing criteria library content".into(),
            path: criteria_dir.clone(),
            source,
        })?;
    tmpfile
        .as_file()
        .sync_all()
        .map_err(|source| AssayError::Io {
            operation: "syncing criteria library file".into(),
            path: criteria_dir.clone(),
            source,
        })?;
    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting criteria library", &final_path, e.error))?;

    Ok(final_path)
}

/// Scan all criteria libraries in `.assay/criteria/`.
///
/// Returns `Ok(vec![])` if the criteria directory does not exist.
/// Skips non-`.toml` files and silently ignores parse errors (consistent with
/// `scan()` in `spec/mod.rs`). Returns libraries sorted by name.
pub fn scan_libraries(assay_dir: &Path) -> Result<Vec<CriteriaLibrary>> {
    let criteria_dir = assay_dir.join("criteria");
    if !criteria_dir.is_dir() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(&criteria_dir).map_err(|source| AssayError::Io {
        operation: "reading criteria directory".into(),
        path: criteria_dir.clone(),
        source,
    })?;

    let mut libs: Vec<CriteriaLibrary> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|e| e == "toml")
                .unwrap_or(false)
        })
        .filter_map(|entry| load_library(&entry.path()).ok())
        .collect();

    libs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(libs)
}

/// Load a criteria library by slug from `.assay/criteria/<slug>.toml`.
///
/// Validates the slug, then attempts to load the file. If the file doesn't
/// exist, scans available slugs and provides a fuzzy-match suggestion.
pub fn load_library_by_slug(assay_dir: &Path, slug: &str) -> Result<CriteriaLibrary> {
    validate_slug(slug)?;

    let criteria_dir = assay_dir.join("criteria");
    let path = criteria_dir.join(format!("{slug}.toml"));

    if !path.exists() {
        // Collect available slugs for fuzzy suggestion
        let available: Vec<String> = if criteria_dir.is_dir() {
            std::fs::read_dir(&criteria_dir)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "toml")
                        .unwrap_or(false)
                })
                .filter_map(|e| {
                    e.path()
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .collect()
        } else {
            vec![]
        };

        let suggestion = crate::spec::find_fuzzy_match(slug, &available);

        return Err(AssayError::LibraryNotFound {
            slug: slug.to_string(),
            criteria_dir,
            suggestion,
        });
    }

    load_library(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_slug tests ────────────────────────────────────────────────────

    #[test]
    fn validate_slug_rust_basics_ok() {
        assert!(validate_slug("rust-basics").is_ok());
    }

    #[test]
    fn validate_slug_underscore_ok() {
        assert!(validate_slug("my_lib").is_ok());
    }

    #[test]
    fn validate_slug_starts_with_digit_ok() {
        assert!(validate_slug("0starts-with-digit").is_ok());
    }

    #[test]
    fn validate_slug_empty_err() {
        let err = validate_slug("").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_uppercase_err() {
        let err = validate_slug("A-Upper").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_path_traversal_err() {
        let err = validate_slug("../evil").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_too_long_err() {
        let slug = "a".repeat(65);
        let err = validate_slug(&slug).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_max_length_ok() {
        let slug = "a".repeat(64);
        assert!(validate_slug(&slug).is_ok(), "64 chars should be accepted");
    }

    #[test]
    fn validate_slug_starts_with_dash_err() {
        let err = validate_slug("-starts-with-dash").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { ref slug, .. } if slug == "-starts-with-dash"),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    // ── library I/O tests ────────────────────────────────────────────────────

    use assay_types::criterion::When;
    use assay_types::{CriteriaLibrary, Criterion};

    fn make_library(name: &str) -> CriteriaLibrary {
        CriteriaLibrary {
            name: name.to_string(),
            description: "Test library".to_string(),
            version: Some("1.0.0".to_string()),
            tags: vec!["test".to_string()],
            criteria: vec![Criterion {
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
        }
    }

    #[test]
    fn load_library_valid_toml() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        let toml_str = toml::to_string_pretty(&lib).expect("serialize");
        let path = tmp.path().join("rust-basics.toml");
        std::fs::write(&path, &toml_str).expect("write");
        let loaded = load_library(&path).expect("load_library");
        assert_eq!(loaded, lib);
    }

    #[test]
    fn load_library_invalid_toml_returns_library_parse_err() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("bad.toml");
        std::fs::write(&path, "not valid toml = [[[").expect("write bad toml");
        let err = load_library(&path).unwrap_err();
        assert!(
            matches!(err, AssayError::LibraryParse { .. }),
            "expected LibraryParse, got: {err:?}"
        );
    }

    #[test]
    fn load_library_nonexistent_returns_io_err() {
        let path = std::path::Path::new("/tmp/nonexistent-assay-test-abc123.toml");
        let err = load_library(path).unwrap_err();
        assert!(
            matches!(err, AssayError::Io { .. }),
            "expected Io error, got: {err:?}"
        );
    }

    #[test]
    fn save_library_valid_slug_writes_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        let path = save_library(tmp.path(), &lib).expect("save_library");
        assert!(path.exists(), "file should exist after save");
        assert_eq!(path, tmp.path().join("criteria/rust-basics.toml"));
    }

    #[test]
    fn save_library_invalid_slug_returns_err_before_io() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut lib = make_library("rust-basics");
        lib.name = "INVALID-SLUG".to_string();
        let err = save_library(tmp.path(), &lib).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
        // No criteria directory should have been created
        assert!(
            !tmp.path().join("criteria").exists(),
            "criteria dir should not be created on slug validation failure"
        );
    }

    #[test]
    fn save_and_load_library_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        let path = save_library(tmp.path(), &lib).expect("save_library");
        let loaded = load_library(&path).expect("load_library");
        assert_eq!(loaded, lib, "roundtrip should preserve all fields");
    }

    #[test]
    fn scan_libraries_missing_dir_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = scan_libraries(tmp.path()).expect("scan_libraries");
        assert!(
            result.is_empty(),
            "should return empty for missing criteria dir"
        );
    }

    #[test]
    fn scan_libraries_returns_all_toml_files_sorted() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib_a = make_library("aaa-lib");
        let lib_b = make_library("bbb-lib");
        save_library(tmp.path(), &lib_b).expect("save bbb");
        save_library(tmp.path(), &lib_a).expect("save aaa");

        let result = scan_libraries(tmp.path()).expect("scan_libraries");
        assert_eq!(result.len(), 2, "should find 2 libraries");
        assert_eq!(result[0].name, "aaa-lib", "should be sorted by name");
        assert_eq!(result[1].name, "bbb-lib");
    }

    #[test]
    fn scan_libraries_skips_non_toml_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let criteria_dir = tmp.path().join("criteria");
        std::fs::create_dir_all(&criteria_dir).expect("create criteria dir");
        std::fs::write(criteria_dir.join("ignored.json"), r#"{"name":"test"}"#)
            .expect("write json");
        std::fs::write(criteria_dir.join("ignored.txt"), "text file").expect("write txt");

        let lib = make_library("valid-lib");
        save_library(tmp.path(), &lib).expect("save valid-lib");

        let result = scan_libraries(tmp.path()).expect("scan_libraries");
        assert_eq!(result.len(), 1, "should only load .toml files");
        assert_eq!(result[0].name, "valid-lib");
    }

    #[test]
    fn load_library_by_slug_existing_returns_library() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        save_library(tmp.path(), &lib).expect("save");
        let loaded = load_library_by_slug(tmp.path(), "rust-basics").expect("load by slug");
        assert_eq!(loaded, lib);
    }

    #[test]
    fn load_library_by_slug_missing_returns_library_not_found_with_suggestion() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        save_library(tmp.path(), &lib).expect("save");

        // Slightly misspelled — should get fuzzy suggestion
        let err = load_library_by_slug(tmp.path(), "rust-bascs").unwrap_err();
        match err {
            AssayError::LibraryNotFound {
                slug, suggestion, ..
            } => {
                assert_eq!(slug, "rust-bascs");
                assert_eq!(
                    suggestion,
                    Some("rust-basics".to_string()),
                    "expected fuzzy suggestion"
                );
            }
            other => panic!("expected LibraryNotFound, got: {other:?}"),
        }
    }
}
