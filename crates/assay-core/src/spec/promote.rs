//! Spec lifecycle promotion.
//!
//! Advances `FeatureSpec.status` through the canonical lifecycle order
//! and persists the change atomically via tempfile-then-rename.

use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;

use crate::error::AssayError;
use crate::spec::{SpecEntry, load_feature_spec, load_spec_entry_with_diagnostics};
use assay_types::feature_spec::SpecStatus;

/// Canonical lifecycle order for `SpecStatus`.
///
/// Exposed for documentation and testing purposes; the order is an implementation
/// detail of this module and should not be depended on by external callers.
pub(crate) const PROMOTION_ORDER: [SpecStatus; 6] = [
    SpecStatus::Draft,
    SpecStatus::Proposed,
    SpecStatus::Planned,
    SpecStatus::InProgress,
    SpecStatus::Verified,
    SpecStatus::Deprecated,
];

/// Returns the next status in the promotion order, or `None` if the
/// current status is terminal for auto-advance (Verified or Deprecated).
pub fn next_status(current: &SpecStatus) -> Option<SpecStatus> {
    let pos = PROMOTION_ORDER.iter().position(|s| s == current)?;
    // Any status at or beyond Verified (the second-to-last) is terminal for auto-advance.
    // Using PROMOTION_ORDER.len() - 2 avoids the magic index 4.
    if pos >= PROMOTION_ORDER.len() - 2 {
        return None;
    }
    Some(PROMOTION_ORDER[pos + 1].clone())
}

/// Promote a spec's lifecycle status.
///
/// - If `target` is `Some`, sets the status directly.
/// - If `target` is `None`, advances to the next status in [`PROMOTION_ORDER`].
///
/// Returns `(old_status, new_status)` on success.
///
/// # Errors
///
/// - Legacy flat specs are not supported (no `spec.toml`).
/// - Directory specs without `spec.toml` (gates-only) are not supported.
/// - Terminal statuses (Verified, Deprecated) without an explicit `target`
///   return an error with guidance to use `--to`.
pub fn promote_spec(
    specs_dir: &Path,
    slug: &str,
    target: Option<SpecStatus>,
) -> crate::Result<(SpecStatus, SpecStatus)> {
    let entry = load_spec_entry_with_diagnostics(slug, specs_dir)?;

    let spec_toml_path = match &entry {
        SpecEntry::Legacy { .. } => {
            return Err(AssayError::Io {
                operation: format!(
                    "promote: '{}' is a legacy flat spec — promotion requires a directory spec with spec.toml",
                    slug
                ),
                path: specs_dir.join(format!("{slug}.toml")),
                source: std::io::Error::new(std::io::ErrorKind::Unsupported, "legacy spec"),
            });
        }
        SpecEntry::Directory {
            spec_path: None, ..
        } => {
            return Err(AssayError::Io {
                operation: format!(
                    "promote: '{}' has no spec.toml — promotion requires a spec.toml with a status field",
                    slug
                ),
                path: specs_dir.join(slug).join("spec.toml"),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "no spec.toml"),
            });
        }
        SpecEntry::Directory {
            spec_path: Some(path),
            ..
        } => path.clone(),
    };

    let mut feature_spec = load_feature_spec(&spec_toml_path)?;
    let old_status = feature_spec.status.clone();

    let new_status = match target {
        Some(t) => t,
        None => next_status(&old_status).ok_or_else(|| AssayError::Io {
            operation: format!(
                "promote: '{}' is already at '{}' — this is a terminal status for auto-advance. Use --to <status> to set a specific status",
                slug, old_status
            ),
            path: spec_toml_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "terminal status"),
        })?,
    };

    feature_spec.status = new_status;

    let toml_str = toml::to_string_pretty(&feature_spec).map_err(|e| AssayError::Io {
        operation: "serializing spec.toml after promotion".into(),
        path: spec_toml_path.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
    })?;

    // Atomic write: tempfile in the same directory, then rename
    let spec_dir = spec_toml_path.parent().unwrap_or_else(|| Path::new("."));

    let mut tmpfile = NamedTempFile::new_in(spec_dir).map_err(|source| AssayError::Io {
        operation: "creating temp file for atomic promote write".into(),
        path: spec_toml_path.clone(),
        source,
    })?;

    tmpfile
        .write_all(toml_str.as_bytes())
        .map_err(|source| AssayError::Io {
            operation: "writing promoted spec.toml".into(),
            path: spec_toml_path.clone(),
            source,
        })?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|source| AssayError::Io {
            operation: "syncing promoted spec.toml".into(),
            path: spec_toml_path.clone(),
            source,
        })?;

    tmpfile
        .persist(&spec_toml_path)
        .map_err(|e| AssayError::Io {
            operation: "persisting promoted spec.toml".into(),
            path: spec_toml_path.clone(),
            source: e.error,
        })?;

    Ok((old_status, feature_spec.status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a directory spec with gates.toml and spec.toml
    fn create_dir_spec(specs_dir: &Path, slug: &str, status: &str) {
        let spec_dir = specs_dir.join(slug);
        fs::create_dir_all(&spec_dir).unwrap();

        // Minimal gates.toml
        fs::write(
            spec_dir.join("gates.toml"),
            format!(
                r#"name = "{slug}"
[[criteria]]
name = "check"
description = "Basic check"
cmd = "echo ok"
"#
            ),
        )
        .unwrap();

        // spec.toml with the given status
        fs::write(
            spec_dir.join("spec.toml"),
            format!(
                r#"name = "{slug}"
status = "{status}"

[[requirements]]
id = "REQ-TEST-001"
title = "Test requirement"
statement = "The system shall do something"
"#
            ),
        )
        .unwrap();
    }

    /// Helper: create a directory spec with only gates.toml (no spec.toml)
    fn create_gates_only_spec(specs_dir: &Path, slug: &str) {
        let spec_dir = specs_dir.join(slug);
        fs::create_dir_all(&spec_dir).unwrap();
        fs::write(
            spec_dir.join("gates.toml"),
            format!(
                r#"name = "{slug}"
[[criteria]]
name = "check"
description = "Basic check"
cmd = "echo ok"
"#
            ),
        )
        .unwrap();
    }

    /// Helper: create a legacy flat spec
    fn create_legacy_spec(specs_dir: &Path, slug: &str) {
        fs::write(
            specs_dir.join(format!("{slug}.toml")),
            format!(
                r#"name = "{slug}"
[[criteria]]
name = "check"
description = "Basic check"
cmd = "echo ok"
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_next_status_draft() {
        assert_eq!(next_status(&SpecStatus::Draft), Some(SpecStatus::Proposed));
    }

    #[test]
    fn test_next_status_proposed() {
        assert_eq!(
            next_status(&SpecStatus::Proposed),
            Some(SpecStatus::Planned)
        );
    }

    #[test]
    fn test_next_status_planned() {
        assert_eq!(
            next_status(&SpecStatus::Planned),
            Some(SpecStatus::InProgress)
        );
    }

    #[test]
    fn test_next_status_in_progress() {
        assert_eq!(
            next_status(&SpecStatus::InProgress),
            Some(SpecStatus::Verified)
        );
    }

    #[test]
    fn test_next_status_verified_is_terminal() {
        assert_eq!(next_status(&SpecStatus::Verified), None);
    }

    #[test]
    fn test_next_status_deprecated_is_terminal() {
        assert_eq!(next_status(&SpecStatus::Deprecated), None);
    }

    #[test]
    fn test_promote_advance_draft_to_proposed() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "draft");

        let (old, new) = promote_spec(specs_dir, "auth-flow", None).unwrap();
        assert_eq!(old, SpecStatus::Draft);
        assert_eq!(new, SpecStatus::Proposed);
    }

    #[test]
    fn test_promote_direct_jump() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "draft");

        let (old, new) = promote_spec(specs_dir, "auth-flow", Some(SpecStatus::Planned)).unwrap();
        assert_eq!(old, SpecStatus::Draft);
        assert_eq!(new, SpecStatus::Planned);
    }

    #[test]
    fn test_promote_terminal_deprecated_error() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "deprecated");

        let err = promote_spec(specs_dir, "auth-flow", None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("terminal"),
            "Expected terminal status error, got: {msg}"
        );
    }

    #[test]
    fn test_promote_terminal_verified_error() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "verified");

        let err = promote_spec(specs_dir, "auth-flow", None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("terminal"),
            "Expected terminal status error, got: {msg}"
        );
    }

    #[test]
    fn test_promote_no_spec_toml_error() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_gates_only_spec(specs_dir, "auth-flow");

        let err = promote_spec(specs_dir, "auth-flow", None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("spec.toml"),
            "Expected spec.toml error, got: {msg}"
        );
    }

    #[test]
    fn test_promote_legacy_flat_error() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_legacy_spec(specs_dir, "auth-flow");

        let err = promote_spec(specs_dir, "auth-flow", None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("legacy"), "Expected legacy error, got: {msg}");
    }

    #[test]
    fn test_promote_round_trip_read_back() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "draft");

        let (_, new) = promote_spec(specs_dir, "auth-flow", None).unwrap();
        assert_eq!(new, SpecStatus::Proposed);

        // Read back from disk and verify
        let spec_toml_path = specs_dir.join("auth-flow").join("spec.toml");
        let content = fs::read_to_string(&spec_toml_path).unwrap();
        let reloaded: assay_types::FeatureSpec = toml::from_str(&content).unwrap();
        assert_eq!(reloaded.status, SpecStatus::Proposed);

        // Verify the serialized format uses kebab-case
        assert!(
            content.contains("status = \"proposed\""),
            "Expected kebab-case status in TOML, got:\n{content}"
        );
    }

    #[test]
    fn test_promote_in_progress_to_verified() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "in-progress");

        let (old, new) = promote_spec(specs_dir, "auth-flow", None).unwrap();
        assert_eq!(old, SpecStatus::InProgress);
        assert_eq!(new, SpecStatus::Verified);
    }

    #[test]
    fn test_promote_to_same_status_succeeds() {
        // --to with the same status as current is allowed; it's a no-op semantically
        // but the caller explicitly requested it, so we respect it.
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "proposed");

        let (old, new) = promote_spec(specs_dir, "auth-flow", Some(SpecStatus::Proposed)).unwrap();
        assert_eq!(old, SpecStatus::Proposed);
        assert_eq!(new, SpecStatus::Proposed);
    }

    #[test]
    fn test_promote_to_deprecated_directly() {
        // --to deprecated should be allowed (explicit jump to end-of-life state)
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "draft");

        let (old, new) =
            promote_spec(specs_dir, "auth-flow", Some(SpecStatus::Deprecated)).unwrap();
        assert_eq!(old, SpecStatus::Draft);
        assert_eq!(new, SpecStatus::Deprecated);
    }

    #[test]
    fn test_promote_unknown_spec_returns_error() {
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        // Create a dummy spec so the directory isn't empty (otherwise we'd get
        // "No specs found" rather than a spec-not-found error for the specific slug).
        create_dir_spec(specs_dir, "other-spec", "draft");

        let err = promote_spec(specs_dir, "nonexistent", None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("nonexistent"),
            "Expected error mentioning the unknown slug, got: {msg}"
        );
    }

    #[test]
    fn test_promote_file_unchanged_on_terminal_error() {
        // When promotion fails (terminal status), the spec.toml must not be modified.
        let tmp = TempDir::new().unwrap();
        let specs_dir = tmp.path();
        create_dir_spec(specs_dir, "auth-flow", "verified");

        let spec_path = specs_dir.join("auth-flow").join("spec.toml");
        let before = fs::read_to_string(&spec_path).unwrap();

        let _ = promote_spec(specs_dir, "auth-flow", None).unwrap_err();

        let after = fs::read_to_string(&spec_path).unwrap();
        assert_eq!(
            before, after,
            "spec.toml must not be modified when promotion fails"
        );
    }
}
