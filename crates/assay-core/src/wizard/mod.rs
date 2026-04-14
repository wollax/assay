//! Guided authoring wizard: pure functions for creating milestones, gate specs,
//! and criteria libraries.
//!
//! This module is the shared foundation for all surfaces (CLI, MCP, TUI). It
//! writes real TOML files to disk using atomic write patterns and returns
//! structured results so callers can surface created paths to users.
//!
//! All functions are pure (no TTY dependency) and callable from a non-TTY
//! context (MCP, tests, background workers).

pub mod criteria;
pub mod gate;
pub mod milestone;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use assay_types::GatesSpec;

use crate::error::{AssayError, Result};

// ── Re-export CriterionInput from assay-types (Plan 01 handoff) ──────────────

/// Single criterion as the user entered it.
///
/// Re-exported from `assay_types` so existing callers at
/// `assay_core::wizard::CriterionInput` keep compiling after the type was
/// re-homed into `assay-types` in Phase 67 Plan 01.
pub use assay_types::CriterionInput;

// ── Re-export milestone wizard items ─────────────────────────────────────────

pub use milestone::{
    WizardChunkInput, WizardInputs, WizardResult, create_from_inputs, create_milestone_from_params,
    create_spec_from_params, slugify,
};

// ── Re-export gate + criteria wizard entry points ─────────────────────────────

pub use criteria::apply_criteria_wizard;
pub use gate::apply_gate_wizard;

// ── plan_quick: transparent 1-chunk milestone ────────────────────────────────

/// Result of a `plan_quick` operation.
pub struct PlanQuickResult {
    /// Path to the created milestone TOML.
    pub milestone_path: PathBuf,
    /// Path to the created gates.toml spec.
    pub spec_path: PathBuf,
    /// The slug used for milestone, chunk, and spec.
    pub slug: String,
}

/// Create a transparent 1-chunk milestone for a flat spec.
///
/// Creates:
/// - A milestone at `.assay/milestones/<slug>.toml` with `quick: true`
/// - A single chunk referencing the spec
/// - A gates.toml spec at `.assay/specs/<slug>/gates.toml`
///
/// The milestone, chunk, and spec all share the same slug, hiding the
/// milestone/chunk abstraction from the solo developer.
pub fn plan_quick(
    assay_dir: &Path,
    name: &str,
    criteria: Vec<assay_types::Criterion>,
) -> Result<PlanQuickResult> {
    let slug = slugify(name);

    let specs_dir = assay_dir.join("specs");
    let milestones_dir = assay_dir.join("milestones");
    std::fs::create_dir_all(&milestones_dir)
        .map_err(|e| AssayError::io("creating milestones directory", &milestones_dir, e))?;

    // Create the gate spec
    let gates = GatesSpec {
        name: slug.clone(),
        description: String::new(),
        status: None,
        uat: None,
        gate: None,
        depends: vec![],
        milestone: Some(slug.clone()),
        order: Some(1),
        extends: None,
        include: vec![],
        preconditions: None,
        criteria,
    };
    let spec_path = write_gate_spec(&gates, &specs_dir)?;

    // Create the quick milestone
    let now = chrono::Utc::now();
    let milestone = assay_types::Milestone {
        slug: slug.clone(),
        name: name.to_string(),
        description: None,
        status: assay_types::MilestoneStatus::InProgress,
        quick: true,
        chunks: vec![assay_types::ChunkRef {
            slug: slug.clone(),
            order: 1,
            depends_on: vec![],
        }],
        completed_chunks: vec![],
        depends_on: vec![],
        pr_branch: None,
        pr_base: None,
        pr_number: None,
        pr_url: None,
        pr_labels: None,
        pr_reviewers: None,
        pr_body_template: None,
        created_at: now,
        updated_at: now,
    };

    crate::milestone::milestone_save(assay_dir, &milestone)?;
    let milestone_path = milestones_dir.join(format!("{slug}.toml"));

    Ok(PlanQuickResult {
        milestone_path,
        spec_path,
        slug,
    })
}

// ── Shared helper: atomic gate spec write ────────────────────────────────────

/// Write a `GatesSpec` to `<specs_dir>/<spec.name>/gates.toml` atomically.
///
/// Creates the spec directory if it doesn't exist. Uses
/// `NamedTempFile::new_in` → `write_all` → `sync_all` → `persist` for
/// crash-safe atomic replacement.
///
/// Returns the final path of the written file.
///
/// This helper is `pub(crate)` — external crates go through
/// [`apply_gate_wizard`] instead.
pub(crate) fn write_gate_spec(spec: &GatesSpec, specs_dir: &Path) -> Result<PathBuf> {
    let chunk_dir = specs_dir.join(&spec.name);
    std::fs::create_dir_all(&chunk_dir)
        .map_err(|e| AssayError::io("creating spec directory", &chunk_dir, e))?;

    let content = toml::to_string_pretty(spec).map_err(|e| AssayError::Io {
        operation: "serializing gates spec TOML".to_string(),
        path: chunk_dir.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
    })?;

    let final_path = chunk_dir.join("gates.toml");

    let mut tmpfile = NamedTempFile::new_in(&chunk_dir)
        .map_err(|e| AssayError::io("creating temp file for gates.toml", &chunk_dir, e))?;

    tmpfile
        .write_all(content.as_bytes())
        .map_err(|e| AssayError::io("writing gates.toml", &final_path, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing gates.toml", &final_path, e))?;

    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting gates.toml", &final_path, e.error))?;

    Ok(final_path)
}
