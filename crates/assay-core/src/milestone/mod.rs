//! Milestone I/O: scan, load, and save milestones under `.assay/milestones/`.
//!
//! Milestones are persisted as TOML files named `<slug>.toml` inside the
//! `.assay/milestones/` directory.  All writes are atomic (NamedTempFile +
//! `sync_all` + `persist`) so a crash mid-write never leaves a corrupt file.

use std::io::Write as _;
use std::path::Path;

use tempfile::NamedTempFile;

use assay_types::Milestone;

use crate::error::{AssayError, Result};
use crate::history::validate_path_component;

/// Scan all milestones in `<assay_dir>/milestones/`.
///
/// Returns `Ok(vec![])` if the directory does not exist.  Each `.toml` file
/// in the directory is parsed as a [`Milestone`]; files with other extensions
/// are silently skipped.  The results are returned sorted by slug.
///
/// # Errors
///
/// Returns [`AssayError::Io`] if a directory entry cannot be read or a `.toml`
/// file cannot be read or parsed.
pub fn milestone_scan(assay_dir: &Path) -> Result<Vec<Milestone>> {
    let milestones_dir = assay_dir.join("milestones");

    if !milestones_dir.exists() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(&milestones_dir)
        .map_err(|e| AssayError::io("reading milestones directory", &milestones_dir, e))?;

    let mut milestones = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| {
            AssayError::io("reading milestones directory entry", &milestones_dir, e)
        })?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AssayError::io("reading milestone", &path, e))?;

        let milestone: Milestone = toml::from_str(&content).map_err(|e| AssayError::Io {
            operation: "parsing milestone TOML".to_string(),
            path: path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        })?;

        milestones.push(milestone);
    }

    milestones.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(milestones)
}

/// Load a single milestone by slug from `<assay_dir>/milestones/<slug>.toml`.
///
/// Validates the slug to prevent path traversal, reads the TOML file, and
/// overwrites the parsed `slug` field with the canonical slug derived from
/// the filename parameter.
///
/// # Errors
///
/// Returns [`AssayError::Io`] if the slug is invalid, the file cannot be read,
/// or the TOML cannot be parsed.
pub fn milestone_load(assay_dir: &Path, slug: &str) -> Result<Milestone> {
    validate_path_component(slug, "milestone slug")?;

    let path = assay_dir.join("milestones").join(format!("{slug}.toml"));

    let content = std::fs::read_to_string(&path)
        .map_err(|e| AssayError::io("reading milestone", &path, e))?;

    let mut milestone: Milestone = toml::from_str(&content).map_err(|e| AssayError::Io {
        operation: "parsing milestone TOML".to_string(),
        path: path.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
    })?;

    // The filename is the canonical source of truth for slug.
    milestone.slug = slug.to_string();

    Ok(milestone)
}

/// Persist a milestone to `<assay_dir>/milestones/<slug>.toml` atomically.
///
/// Creates `<assay_dir>/milestones/` if it does not exist. Writes via a
/// temporary file, calls `sync_all`, then renames to the final path so a
/// crash mid-write never leaves a partial file.
///
/// # Errors
///
/// Returns [`AssayError::Io`] if the slug is invalid, the directory cannot be
/// created, serialization fails, or any I/O step fails.
pub fn milestone_save(assay_dir: &Path, milestone: &Milestone) -> Result<()> {
    validate_path_component(&milestone.slug, "milestone slug")?;

    let milestones_dir = assay_dir.join("milestones");
    std::fs::create_dir_all(&milestones_dir)
        .map_err(|e| AssayError::io("creating milestones directory", &milestones_dir, e))?;

    let final_path = milestones_dir.join(format!("{}.toml", milestone.slug));

    let content = toml::to_string(milestone).map_err(|e| AssayError::Io {
        operation: "serializing milestone TOML".to_string(),
        path: final_path.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
    })?;

    let mut tmpfile = NamedTempFile::new_in(&milestones_dir)
        .map_err(|e| AssayError::io("creating temp file for milestone", &milestones_dir, e))?;

    tmpfile
        .write_all(content.as_bytes())
        .map_err(|e| AssayError::io("writing milestone", &final_path, e))?;

    tmpfile
        .flush()
        .map_err(|e| AssayError::io("flushing milestone", &final_path, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing milestone", &final_path, e))?;

    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting milestone", &final_path, e.error))?;

    Ok(())
}
