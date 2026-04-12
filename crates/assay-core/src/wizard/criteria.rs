//! Core entry point for the criteria-library wizard. Surface-agnostic.
//!
//! Callable from CLI, MCP, TUI, or tests — no TTY dependency.

use std::path::Path;

use assay_types::criterion::When;
use assay_types::{
    CriteriaLibrary, CriteriaWizardInput, CriteriaWizardOutput, Criterion, CriterionInput,
};

use crate::error::{AssayError, Result};
use crate::spec::compose;

/// Create or replace a criteria library from wizard input.
///
/// Slug validation happens **before** any I/O. The overwrite check is performed
/// by this function; `compose::save_library` always writes unconditionally.
///
/// # Errors
///
/// - [`AssayError::InvalidSlug`] if `input.name` is not a valid slug.
/// - [`AssayError::Io`] with `source.kind() == AlreadyExists` if
///   `overwrite=false` and the target library file already exists.
/// - [`AssayError::Io`] for any other I/O failure.
pub fn apply_criteria_wizard(
    input: &CriteriaWizardInput,
    assay_dir: &Path,
) -> Result<CriteriaWizardOutput> {
    // 1. Validate slug BEFORE any I/O (save_library also validates, but fail-fast
    //    at the wizard layer gives better error attribution).
    compose::validate_slug(&input.name)?;

    // 2. Collision check — save_library unconditionally overwrites, so we must
    //    guard here if the caller did not opt in to overwriting.
    let target = assay_dir
        .join("criteria")
        .join(format!("{}.toml", &input.name));
    if target.exists() && !input.overwrite {
        return Err(AssayError::Io {
            operation: format!("criteria library '{}' already exists", input.name),
            path: target,
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "library already exists; pass overwrite=true to replace",
            ),
        });
    }

    // 3. Build CriteriaLibrary from input.
    let library = CriteriaLibrary {
        name: input.name.clone(),
        description: input.description.clone(),
        version: input.version.clone(),
        tags: input.tags.clone(),
        criteria: input.criteria.iter().map(criterion_from_input).collect(),
    };

    // 4. save_library handles atomic write + directory creation.
    let path = compose::save_library(assay_dir, &library)?;

    Ok(CriteriaWizardOutput { path, library })
}

/// Map a `CriterionInput` to a `Criterion` with explicit defaults for unused fields.
fn criterion_from_input(input: &CriterionInput) -> Criterion {
    Criterion {
        name: input.name.clone(),
        description: input.description.clone(),
        cmd: input.cmd.clone(),
        path: None,
        timeout: None,
        enforcement: None,
        kind: None,
        prompt: None,
        requirements: vec![],
        when: When::default(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use tempfile::TempDir;

    fn assay_dir(tmp: &TempDir) -> PathBuf {
        tmp.path().to_path_buf()
    }

    fn minimal_input(name: &str) -> CriteriaWizardInput {
        CriteriaWizardInput {
            name: name.to_string(),
            description: String::new(),
            version: None,
            tags: vec![],
            criteria: vec![],
            overwrite: false,
        }
    }

    // ── Test 4: slug_rejected (unit) ─────────────────────────────────────────

    #[test]
    fn slug_rejected_invalid() {
        let tmp = TempDir::new().unwrap();
        let mut input = minimal_input("placeholder");
        input.name = "../evil".to_string();
        let err =
            apply_criteria_wizard(&input, &assay_dir(&tmp)).expect_err("should reject ../evil");
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
        // No criteria directory should exist.
        assert!(
            !assay_dir(&tmp).join("criteria").exists(),
            "criteria dir must not be created on invalid slug"
        );
    }

    // ── Test 5: minimal_payload (unit) ───────────────────────────────────────

    #[test]
    fn minimal_payload() {
        let tmp = TempDir::new().unwrap();
        let input = minimal_input("my-lib");
        let out =
            apply_criteria_wizard(&input, &assay_dir(&tmp)).expect("minimal input should succeed");
        assert!(out.path.exists(), "library file should exist");
        assert_eq!(out.library.name, "my-lib");
        assert!(
            out.library.criteria.is_empty(),
            "empty criteria must be allowed"
        );
    }
}
