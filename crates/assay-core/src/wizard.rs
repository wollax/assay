//! Guided authoring wizard: pure functions for creating milestones and specs.
//!
//! This module is the shared foundation for both the CLI (`assay plan`) and the
//! MCP tools (`milestone_create`, `spec_create`). It writes real TOML files to
//! disk using the established atomic write patterns from [`crate::milestone`].
//!
//! All functions are pure (no TTY dependency) and return structured results so
//! callers can surface created paths to users.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tempfile::NamedTempFile;

use assay_types::{ChunkRef, Criterion, GatesSpec, Milestone, MilestoneStatus};

use crate::error::{AssayError, Result};
use crate::history::validate_path_component;
use crate::milestone::{milestone_load, milestone_save};

// ── Public types ─────────────────────────────────────────────────────────────

/// A single criterion string for a wizard chunk input.
///
/// The string is used as the criterion's `name`; `description` defaults to empty.
/// This keeps the wizard input surface minimal while still producing valid `GatesSpec`
/// criterion entries.
pub struct CriterionInput {
    /// Human-readable criterion name.
    pub name: String,
    /// Detailed description of what this criterion checks.
    pub description: String,
    /// Optional shell command.
    pub cmd: Option<String>,
}

/// A single chunk (spec) within a wizard input.
///
/// The `slug` is used directly as the spec directory name and the `GatesSpec.name`
/// field. `criteria` are plain strings used as criterion names.
#[derive(Debug)]
pub struct WizardChunkInput {
    /// Slug for this chunk (used as the spec directory name).
    pub slug: String,
    /// Human-readable display name.
    pub name: String,
    /// Criterion names for this chunk. Each string becomes a [`Criterion`] with
    /// `name = string` and `description = ""`.
    pub criteria: Vec<String>,
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
        let gates_path = write_gates_toml(
            &chunk.slug,
            &inputs.slug,
            Some(i as u32),
            &chunk.criteria,
            specs_dir,
        )?;
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
        .map(|(s, order)| ChunkRef { slug: s, order })
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
/// `criteria` is a list of criterion name strings; each becomes a [`Criterion`] with
/// `name = string` and `description = ""`. Pass an empty `Vec` when no criteria are needed.
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
    criteria: Vec<String>,
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
    let gates_path = write_gates_toml(
        slug,
        &ms_slug_str,
        milestone_slug.map(|_| 0u32),
        &criteria,
        specs_dir,
    )?;

    // Patch milestone's chunks if linked.
    if let Some(ms) = milestone_slug {
        let mut milestone = milestone_load(assay_dir, ms)?;
        let order = milestone.chunks.len() as u32;
        milestone.chunks.push(ChunkRef {
            slug: slug.to_string(),
            order,
        });
        milestone.updated_at = Utc::now();
        milestone_save(assay_dir, &milestone)?;
    }

    Ok(gates_path)
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Write a `gates.toml` file atomically to `<specs_dir>/<slug>/gates.toml`.
///
/// Creates the directory if it doesn't exist. Criteria strings become
/// [`Criterion`] entries with `name = string` and `description = ""`.
fn write_gates_toml(
    slug: &str,
    milestone_slug: &str,
    order: Option<u32>,
    criteria_names: &[String],
    specs_dir: &Path,
) -> Result<PathBuf> {
    let chunk_dir = specs_dir.join(slug);
    std::fs::create_dir_all(&chunk_dir)
        .map_err(|e| AssayError::io("creating spec directory", &chunk_dir, e))?;

    let criteria: Vec<Criterion> = criteria_names
        .iter()
        .map(|name| Criterion {
            name: name.clone(),
            description: String::new(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        })
        .collect();

    let spec = GatesSpec {
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
        criteria,
    };

    let content = toml::to_string_pretty(&spec).map_err(|e| AssayError::Io {
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
