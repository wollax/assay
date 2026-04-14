//! Core entry point for the gate-authoring wizard. Surface-agnostic.
//!
//! Callable from CLI, MCP, TUI, or tests — no TTY dependency.

use std::path::Path;

use assay_types::criterion::When;
use assay_types::{Criterion, CriterionInput, GateWizardInput, GateWizardOutput, GatesSpec};

use crate::error::{AssayError, Result};
use crate::spec::compose;

/// Create or replace a gate spec (`gates.toml`) from wizard input.
///
/// All slug validation happens **before** any I/O, so callers get a clean
/// [`AssayError::InvalidSlug`] with no partial writes if validation fails.
///
/// # Errors
///
/// - [`AssayError::InvalidSlug`] if `input.slug`, `input.extends`, or any
///   element of `input.include` is not a valid slug.
/// - [`AssayError::Io`] with `source.kind() == AlreadyExists` if
///   `overwrite=false` and the target `gates.toml` already exists.
/// - [`AssayError::Io`] for any other I/O failure.
pub fn apply_gate_wizard(
    input: &GateWizardInput,
    _assay_dir: &Path,
    specs_dir: &Path,
) -> Result<GateWizardOutput> {
    // 1. Validate all slugs BEFORE any I/O.
    compose::validate_slug(&input.slug)?;
    if let Some(ref parent) = input.extends {
        compose::validate_slug(parent)?;
    }
    for lib in &input.include {
        compose::validate_slug(lib)?;
    }

    // 2. Check collision if not overwriting.
    let gate_path = specs_dir.join(&input.slug).join("gates.toml");
    if gate_path.exists() && !input.overwrite {
        return Err(AssayError::Io {
            operation: format!("gate '{}' already exists", input.slug),
            path: gate_path,
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "gate file already exists; pass overwrite=true to replace",
            ),
        });
    }

    // 3. Build GatesSpec from input.
    let spec = build_gates_spec(input);

    // 4. Atomic write via shared helper from wizard/mod.rs.
    let path = super::write_gate_spec(&spec, specs_dir)?;

    Ok(GateWizardOutput { path, spec })
}

/// Build a `GatesSpec` from `GateWizardInput`, mapping `CriterionInput` → `Criterion`.
fn build_gates_spec(input: &GateWizardInput) -> GatesSpec {
    let criteria: Vec<Criterion> = input.criteria.iter().map(criterion_from_input).collect();

    GatesSpec {
        name: input.slug.clone(),
        description: input.description.clone().unwrap_or_default(),
        status: None,
        uat: None,
        gate: None,
        depends: vec![],
        milestone: None,
        order: None,
        extends: input.extends.clone(),
        include: input.include.clone(),
        preconditions: input.preconditions.clone(),
        criteria,
    }
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

    fn specs_dir(tmp: &TempDir) -> PathBuf {
        tmp.path().join("specs")
    }

    fn assay_dir(tmp: &TempDir) -> PathBuf {
        tmp.path().join(".assay")
    }

    fn minimal_input(slug: &str) -> GateWizardInput {
        GateWizardInput {
            slug: slug.to_string(),
            description: None,
            extends: None,
            include: vec![],
            criteria: vec![],
            preconditions: None,
            overwrite: false,
        }
    }

    // ── Test 4: slug rejection (unit) ─────────────────────────────────────────

    #[test]
    fn slug_rejected_empty() {
        let tmp = TempDir::new().unwrap();
        let mut input = minimal_input("placeholder");
        input.slug = String::new();
        let err = apply_gate_wizard(&input, &assay_dir(&tmp), &specs_dir(&tmp)).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
        // No directory should have been created.
        assert!(
            !specs_dir(&tmp).exists(),
            "specs_dir must not be created on invalid slug"
        );
    }

    #[test]
    fn slug_rejected_path_traversal() {
        let tmp = TempDir::new().unwrap();
        let mut input = minimal_input("../evil");
        input.slug = "../evil".to_string();
        let err = apply_gate_wizard(&input, &assay_dir(&tmp), &specs_dir(&tmp)).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
        assert!(
            !specs_dir(&tmp).exists(),
            "specs_dir must not be created on invalid slug"
        );
    }

    #[test]
    fn slug_rejected_uppercase() {
        let tmp = TempDir::new().unwrap();
        let input = minimal_input("BadCase");
        let err = apply_gate_wizard(&input, &assay_dir(&tmp), &specs_dir(&tmp)).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug for 'BadCase', got: {err:?}"
        );
        assert!(!specs_dir(&tmp).exists());
    }

    // ── Test 5: extends slug rejected (unit) ─────────────────────────────────

    #[test]
    fn extends_slug_rejected() {
        let tmp = TempDir::new().unwrap();
        let mut input = minimal_input("valid-slug");
        input.extends = Some("../nope".to_string());
        let err = apply_gate_wizard(&input, &assay_dir(&tmp), &specs_dir(&tmp)).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug for extends, got: {err:?}"
        );
        assert!(!specs_dir(&tmp).exists());
    }

    // ── Test 6: include slug rejected (unit) ─────────────────────────────────

    #[test]
    fn include_slug_rejected() {
        let tmp = TempDir::new().unwrap();
        let mut input = minimal_input("valid-slug");
        input.include = vec!["good".to_string(), "../evil".to_string()];
        let err = apply_gate_wizard(&input, &assay_dir(&tmp), &specs_dir(&tmp)).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug for include, got: {err:?}"
        );
        assert!(!specs_dir(&tmp).exists());
    }
}
