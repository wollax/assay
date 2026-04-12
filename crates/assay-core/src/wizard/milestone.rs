//! Milestone wizard: pure functions for creating milestones and linked specs.
//!
//! All public items from the original `wizard.rs` that relate to milestone/chunk
//! authoring live here. They are re-exported from `wizard/mod.rs` so external
//! callers at `assay_core::wizard::*` remain unaffected.

use std::path::{Path, PathBuf};

use chrono::Utc;

use assay_types::criterion::When;
use assay_types::{ChunkRef, Criterion, GatesSpec, Milestone, MilestoneStatus};

use crate::error::{AssayError, Result};
use crate::history::validate_path_component;
use crate::milestone::{milestone_load, milestone_save};
use crate::wizard::CriterionInput;

// ── Public types ─────────────────────────────────────────────────────────────

/// A single chunk (spec) within a wizard input.
///
/// The `slug` is used directly as the spec directory name and the `GatesSpec.name`
/// field.
#[derive(Debug)]
pub struct WizardChunkInput {
    /// Slug for this chunk (used as the spec directory name).
    pub slug: String,
    /// Human-readable display name.
    pub name: String,
    /// Criteria for this chunk. Each [`CriterionInput`] becomes a [`Criterion`]
    /// in the generated `gates.toml`, preserving name, description, and optional cmd.
    pub criteria: Vec<CriterionInput>,
}

/// Top-level input for the guided wizard.
///
/// The `slug` is used directly as the milestone slug; `name` is the display name.
/// All chunk slugs must be pre-validated by the caller (or derived by [`slugify`]).
#[derive(Debug)]
pub struct WizardInputs {
    /// Slug for the milestone (used as the TOML filename without extension).
    pub slug: String,
    /// Human-readable display name for the milestone.
    pub name: String,
    /// Optional longer description.
    pub description: Option<String>,
    /// Ordered list of chunks to create as specs.
    pub chunks: Vec<WizardChunkInput>,
}

/// Result of a successful [`create_from_inputs`] call.
///
/// Callers can surface these paths to users to confirm what was written.
pub struct WizardResult {
    /// Path to the written milestone TOML file.
    pub milestone_path: PathBuf,
    /// Paths to the written `gates.toml` files (one per chunk, in order).
    pub spec_paths: Vec<PathBuf>,
}

// ── slugify ───────────────────────────────────────────────────────────────────

/// Convert a human-readable name to a safe slug.
///
/// Lowercases the string, collapses any run of non-`[a-z0-9]` characters
/// to a single `-`, and strips leading/trailing hyphens.
///
/// # Examples
///
/// ```
/// use assay_core::wizard::slugify;
///
/// assert_eq!(slugify("My Feature!"), "my-feature");
/// assert_eq!(slugify("My Feature 2"), "my-feature-2");
/// assert_eq!(slugify("  leading and trailing  "), "leading-and-trailing");
/// ```
///
/// # Panics
///
/// Panics if the result is empty after stripping (i.e. the input contained no
/// alphanumeric characters). Slugify is intended for user-provided names, not
/// untrusted input; callers should validate names before calling this.
pub fn slugify(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut slug = String::with_capacity(lower.len());
    let mut prev_hyphen = false;

    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    // Strip leading/trailing hyphens.
    let slug = slug.trim_matches('-').to_string();

    assert!(
        !slug.is_empty(),
        "slugify: result is empty for input {s:?}; input must contain at least one alphanumeric character"
    );

    slug
}

// ── create_from_inputs ────────────────────────────────────────────────────────

/// Create a milestone and its associated chunk specs from wizard inputs.
///
/// Writes `<assay_dir>/milestones/<slug>.toml` and
/// `<specs_dir>/<chunk_slug>/gates.toml` for each chunk. All writes are atomic.
///
/// # Errors
///
/// - [`AssayError::Io`] if the milestone slug already exists.
/// - [`AssayError::Io`] for any I/O failure during directory creation or file writing.
pub fn create_from_inputs(
    inputs: &WizardInputs,
    assay_dir: &Path,
    specs_dir: &Path,
) -> Result<WizardResult> {
    validate_path_component(&inputs.slug, "milestone slug")?;

    // Reject slug collision.
    let milestone_file = assay_dir
        .join("milestones")
        .join(format!("{}.toml", inputs.slug));
    if milestone_file.exists() {
        return Err(AssayError::Io {
            operation: format!("milestone '{}' already exists", inputs.slug),
            path: milestone_file.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("milestone '{}' already exists", inputs.slug),
            ),
        });
    }

    // Build chunk refs for the milestone.
    let chunk_refs: Vec<ChunkRef> = inputs
        .chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| ChunkRef {
            slug: chunk.slug.clone(),
            order: i as u32,
            depends_on: vec![],
        })
        .collect();

    // Build and save the milestone.
    let now = Utc::now();
    let milestone = Milestone {
        slug: inputs.slug.clone(),
        name: inputs.name.clone(),
        description: inputs.description.clone(),
        status: MilestoneStatus::Draft,
        chunks: chunk_refs,
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
    milestone_save(assay_dir, &milestone)?;

    // Write each chunk spec.
    let mut spec_paths = Vec::with_capacity(inputs.chunks.len());
    for (i, chunk) in inputs.chunks.iter().enumerate() {
        let spec =
            build_milestone_gate_spec(&chunk.slug, &inputs.slug, Some(i as u32), &chunk.criteria);
        let gates_path = super::write_gate_spec(&spec, specs_dir)?;
        spec_paths.push(gates_path);
    }

    Ok(WizardResult {
        milestone_path: milestone_file,
        spec_paths,
    })
}

// ── create_milestone_from_params ──────────────────────────────────────────────

/// Create a milestone from explicit parameters (used by the MCP `milestone_create` tool).
///
/// `chunks` is a list of `(slug, order)` pairs that become [`ChunkRef`] entries.
///
/// # Errors
///
/// - [`AssayError::Io`] if the slug is invalid or already exists.
pub fn create_milestone_from_params(
    slug: &str,
    name: &str,
    description: Option<&str>,
    chunks: Vec<(String, u32)>,
    assay_dir: &Path,
) -> Result<Milestone> {
    validate_path_component(slug, "milestone slug")?;

    let milestone_file = assay_dir.join("milestones").join(format!("{slug}.toml"));
    if milestone_file.exists() {
        return Err(AssayError::Io {
            operation: format!("milestone '{slug}' already exists"),
            path: milestone_file,
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("milestone '{slug}' already exists"),
            ),
        });
    }

    let chunk_refs: Vec<ChunkRef> = chunks
        .into_iter()
        .map(|(s, order)| ChunkRef {
            slug: s,
            order,
            depends_on: vec![],
        })
        .collect();

    let now = Utc::now();
    let milestone = Milestone {
        slug: slug.to_string(),
        name: name.to_string(),
        description: description.map(str::to_string),
        status: MilestoneStatus::Draft,
        chunks: chunk_refs,
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

    milestone_save(assay_dir, &milestone)?;
    Ok(milestone)
}

// ── create_spec_from_params ───────────────────────────────────────────────────

/// Create a single spec (chunk) from explicit parameters and optionally link it to a milestone.
///
/// Writes `<specs_dir>/<slug>/gates.toml` atomically. If `milestone_slug` is provided,
/// the spec is linked to the milestone and the milestone's `chunks` list is updated.
///
/// `criteria` is a list of [`CriterionInput`] values; each becomes a [`Criterion`] in
/// the generated `gates.toml`, preserving name, description, and optional cmd.
/// Pass an empty `Vec` when no criteria are needed.
///
/// # Errors
///
/// - [`AssayError::Io`] if the spec directory already exists.
/// - [`AssayError::Io`] if `milestone_slug` is provided but no matching milestone exists.
pub fn create_spec_from_params(
    slug: &str,
    _name: &str,
    milestone_slug: Option<&str>,
    assay_dir: &Path,
    specs_dir: &Path,
    criteria: Vec<CriterionInput>,
) -> Result<PathBuf> {
    validate_path_component(slug, "spec slug")?;

    // Reject duplicate spec directory.
    let spec_dir = specs_dir.join(slug);
    if spec_dir.exists() {
        return Err(AssayError::Io {
            operation: format!("spec directory '{slug}' already exists"),
            path: spec_dir.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("spec directory '{slug}' already exists"),
            ),
        });
    }

    // Verify (and load) the milestone if requested — fail early if it doesn't exist.
    if let Some(ms) = milestone_slug {
        milestone_load(assay_dir, ms)?;
    }

    // Write the gates.toml.
    let ms_slug_str = milestone_slug.unwrap_or("").to_string();
    let spec =
        build_milestone_gate_spec(slug, &ms_slug_str, milestone_slug.map(|_| 0u32), &criteria);
    let gates_path = super::write_gate_spec(&spec, specs_dir)?;

    // Patch milestone's chunks if linked.
    if let Some(ms) = milestone_slug {
        let mut milestone = milestone_load(assay_dir, ms)?;
        let order = milestone.chunks.len() as u32;
        milestone.chunks.push(ChunkRef {
            slug: slug.to_string(),
            order,
            depends_on: vec![],
        });
        milestone.updated_at = Utc::now();
        milestone_save(assay_dir, &milestone)?;
    }

    Ok(gates_path)
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Build a `GatesSpec` for a milestone chunk, mapping `CriterionInput` to `Criterion`.
fn build_milestone_gate_spec(
    slug: &str,
    milestone_slug: &str,
    order: Option<u32>,
    criteria_inputs: &[CriterionInput],
) -> GatesSpec {
    let criteria: Vec<Criterion> = criteria_inputs
        .iter()
        .map(|input| Criterion {
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
        })
        .collect();

    GatesSpec {
        name: slug.to_string(),
        description: String::new(),
        gate: None,
        depends: vec![],
        milestone: if milestone_slug.is_empty() {
            None
        } else {
            Some(milestone_slug.to_string())
        },
        order,
        extends: None,
        include: vec![],
        preconditions: None,
        criteria,
    }
}
