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
// NOTE: Re-exports added in Tasks 2 and 3 after the functions are implemented.

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
