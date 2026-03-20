//! Cycle state machine for milestone-driven development.
//!
//! Provides [`CycleStatus`], [`active_chunk`], [`cycle_status`],
//! [`milestone_phase_transition`], and [`cycle_advance`] — the core business
//! logic for advancing a milestone through its chunk sequence.
//!
//! All functions are pure sync (D007: sync core). The MCP layer wraps
//! [`cycle_advance`] in `spawn_blocking` when called from an async context.

use std::io;
use std::path::Path;

use chrono::Utc;
use serde::Serialize;

use assay_types::{ChunkRef, Milestone, MilestoneStatus};

use crate::error::{AssayError, Result};
use crate::gate::evaluate_all_gates;
use crate::history::validate_path_component;
use crate::milestone::{milestone_load, milestone_save, milestone_scan};
use crate::spec::{SpecEntry, load_spec_entry_with_diagnostics};

// ── CycleStatus ──────────────────────────────────────────────────────────────

/// Snapshot of where a milestone stands in its chunk-by-chunk advancement.
///
/// Returned by [`cycle_status`] and [`cycle_advance`].
#[derive(Debug, Clone, Serialize)]
pub struct CycleStatus {
    /// Slug of the milestone currently in progress.
    pub milestone_slug: String,
    /// Human-readable name of the milestone.
    pub milestone_name: String,
    /// Current workflow phase.
    pub phase: MilestoneStatus,
    /// Slug of the next chunk awaiting gate evaluation, or `None` if all
    /// chunks are complete (milestone is ready to transition to `Verify`).
    pub active_chunk_slug: Option<String>,
    /// Number of chunks already marked complete.
    pub completed_count: usize,
    /// Total number of chunks in the milestone.
    pub total_count: usize,
}

// ── active_chunk ─────────────────────────────────────────────────────────────

/// Return the next incomplete chunk in order, or `None` if all are done.
///
/// Chunks are sorted ascending by [`ChunkRef::order`] (a required `u32`).
/// The first chunk whose slug does not appear in `milestone.completed_chunks`
/// is returned.  Ties in `order` preserve insertion order (stable sort).
pub fn active_chunk(milestone: &Milestone) -> Option<&ChunkRef> {
    let mut ordered: Vec<&ChunkRef> = milestone.chunks.iter().collect();
    ordered.sort_by_key(|c| c.order);
    ordered
        .into_iter()
        .find(|c| !milestone.completed_chunks.contains(&c.slug))
}

// ── cycle_status ─────────────────────────────────────────────────────────────

/// Return the current cycle status for the first `InProgress` milestone found.
///
/// Returns `Ok(None)` when there are no milestones or none are `InProgress`.
/// Milestones are scanned and sorted alphabetically; the first `InProgress`
/// one wins.
///
/// # Errors
///
/// Returns [`AssayError::Io`] if the milestone directory cannot be read.
pub fn cycle_status(assay_dir: &Path) -> Result<Option<CycleStatus>> {
    let milestones = milestone_scan(assay_dir)?;

    let milestone = milestones
        .into_iter()
        .find(|m| m.status == MilestoneStatus::InProgress);

    let Some(milestone) = milestone else {
        return Ok(None);
    };

    let active_chunk_slug = active_chunk(&milestone).map(|c| c.slug.clone());
    let completed_count = milestone.completed_chunks.len();
    let total_count = milestone.chunks.len();

    Ok(Some(CycleStatus {
        milestone_slug: milestone.slug,
        milestone_name: milestone.name,
        phase: milestone.status,
        active_chunk_slug,
        completed_count,
        total_count,
    }))
}

// ── milestone_phase_transition ────────────────────────────────────────────────

/// Attempt a phase transition on `milestone`, updating `status` and
/// `updated_at` on success.
///
/// Valid transitions and their preconditions:
///
/// | From        | To         | Precondition                              |
/// |-------------|------------|-------------------------------------------|
/// | Draft       | InProgress | `milestone.chunks` must not be empty      |
/// | InProgress  | Verify     | `active_chunk(milestone)` must be `None`  |
/// | Verify      | Complete   | Always allowed                            |
///
/// All other transitions return [`AssayError::Io`] with a descriptive message.
///
/// # Errors
///
/// Returns [`AssayError::Io`] for invalid transitions or unmet preconditions.
pub fn milestone_phase_transition(milestone: &mut Milestone, next: MilestoneStatus) -> Result<()> {
    use MilestoneStatus::*;

    let current = milestone.status;

    match (&current, &next) {
        (Draft, InProgress) => {
            if milestone.chunks.is_empty() {
                return Err(AssayError::Io {
                    operation: "milestone phase transition".to_string(),
                    path: std::path::PathBuf::from(&milestone.slug),
                    source: io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "cannot transition milestone '{}' from Draft to InProgress: \
                             milestone has no chunks (add at least one chunk first)",
                            milestone.slug
                        ),
                    ),
                });
            }
        }

        (InProgress, Verify) => {
            if active_chunk(milestone).is_some() {
                return Err(AssayError::Io {
                    operation: "milestone phase transition".to_string(),
                    path: std::path::PathBuf::from(&milestone.slug),
                    source: io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "cannot transition milestone '{}' from InProgress to Verify: \
                             there is still an active chunk — advance all chunks first",
                            milestone.slug
                        ),
                    ),
                });
            }
        }

        (Verify, Complete) => {
            // Always allowed.
        }

        _ => {
            return Err(AssayError::Io {
                operation: "milestone phase transition".to_string(),
                path: std::path::PathBuf::from(&milestone.slug),
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "invalid milestone phase transition: cannot transition from \
                         {current:?} to {next:?}",
                    ),
                ),
            });
        }
    }

    milestone.status = next;
    milestone.updated_at = Utc::now();
    Ok(())
}

// ── cycle_advance ─────────────────────────────────────────────────────────────

/// Evaluate gates for the active chunk and, on success, mark it complete.
///
/// ## Algorithm
///
/// 1. Locate the target milestone: if `milestone_slug` is `Some`, load it by
///    slug; otherwise scan for the first `InProgress` milestone.
/// 2. Verify the milestone's status is `InProgress`.
/// 3. Identify the active chunk (lowest-order incomplete chunk).
/// 4. Load the directory spec for the active chunk from `specs_dir`.
/// 5. Evaluate all gates synchronously.
/// 6. If any *required* gate fails, return `Err` without modifying state.
/// 7. Push the chunk slug into `milestone.completed_chunks`.
/// 8. If no active chunk remains after step 7, call
///    [`milestone_phase_transition`] to advance to `Verify`.
/// 9. Save the milestone atomically via [`milestone_save`].
/// 10. Return the updated [`CycleStatus`].
///
/// # Errors
///
/// Returns [`AssayError::Io`] for:
/// - No `InProgress` milestone found (when `milestone_slug` is `None`)
/// - Milestone not in `InProgress` status
/// - No active chunk (all complete but not yet transitioned)
/// - Spec not found for the active chunk
/// - Legacy spec used for a milestone chunk (directory specs required)
/// - Required gates failed (milestone state not modified)
/// - I/O error saving the milestone
pub fn cycle_advance(
    assay_dir: &Path,
    specs_dir: &Path,
    working_dir: &Path,
    milestone_slug: Option<&str>,
) -> Result<CycleStatus> {
    // ── Step 1: Locate milestone ──────────────────────────────────────────
    let mut milestone = match milestone_slug {
        Some(slug) => milestone_load(assay_dir, slug)?,
        None => {
            let milestones = milestone_scan(assay_dir)?;
            milestones
                .into_iter()
                .find(|m| m.status == MilestoneStatus::InProgress)
                .ok_or_else(|| AssayError::Io {
                    operation: "cycle_advance".to_string(),
                    path: assay_dir.to_path_buf(),
                    source: io::Error::new(
                        io::ErrorKind::NotFound,
                        "no active (in_progress) milestone found",
                    ),
                })?
        }
    };

    // ── Step 2: Verify status ─────────────────────────────────────────────
    if milestone.status != MilestoneStatus::InProgress {
        return Err(AssayError::Io {
            operation: "cycle_advance".to_string(),
            path: assay_dir.to_path_buf(),
            source: io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "milestone '{}' is not in_progress (status: {:?})",
                    milestone.slug, milestone.status
                ),
            ),
        });
    }

    // ── Step 3: Identify active chunk ─────────────────────────────────────
    let active_slug = active_chunk(&milestone)
        .map(|c| c.slug.clone())
        .ok_or_else(|| AssayError::Io {
            operation: "cycle_advance".to_string(),
            path: assay_dir.to_path_buf(),
            source: io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "milestone '{}' has no active chunk (all chunks may be complete \
                     — call milestone_phase_transition)",
                    milestone.slug
                ),
            ),
        })?;

    // ── Step 4: Load spec ─────────────────────────────────────────────────
    validate_path_component(&active_slug, "chunk slug")?;

    let spec_entry = load_spec_entry_with_diagnostics(&active_slug, specs_dir)?;

    let gates = match spec_entry {
        SpecEntry::Directory { gates, .. } => gates,
        SpecEntry::Legacy { slug, .. } => {
            return Err(AssayError::Io {
                operation: "cycle_advance".to_string(),
                path: specs_dir.join(format!("{slug}.toml")),
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "chunk '{slug}' is a legacy spec; directory specs required for \
                         milestone chunks (create specs/{slug}/gates.toml instead)"
                    ),
                ),
            });
        }
    };

    // ── Step 5: Evaluate gates ────────────────────────────────────────────
    let summary = evaluate_all_gates(&gates, working_dir, None, None);

    // ── Step 6: Check result ──────────────────────────────────────────────
    if summary.enforcement.required_failed > 0 {
        return Err(AssayError::Io {
            operation: "cycle_advance".to_string(),
            path: specs_dir.join(&active_slug),
            source: io::Error::other(format!(
                "chunk '{}' gates failed: {} required criteria did not pass",
                active_slug, summary.enforcement.required_failed
            )),
        });
    }

    // ── Step 7: Mark complete ─────────────────────────────────────────────
    milestone.completed_chunks.push(active_slug);
    milestone.updated_at = Utc::now();

    // ── Step 8: Transition to Verify if all done ──────────────────────────
    if active_chunk(&milestone).is_none() {
        milestone_phase_transition(&mut milestone, MilestoneStatus::Verify)?;
    }

    // ── Step 9: Save ──────────────────────────────────────────────────────
    milestone_save(assay_dir, &milestone)?;

    // ── Step 10: Return status ────────────────────────────────────────────
    let active_chunk_slug = active_chunk(&milestone).map(|c| c.slug.clone());
    let completed_count = milestone.completed_chunks.len();
    let total_count = milestone.chunks.len();

    Ok(CycleStatus {
        milestone_slug: milestone.slug,
        milestone_name: milestone.name,
        phase: milestone.status,
        active_chunk_slug,
        completed_count,
        total_count,
    })
}
