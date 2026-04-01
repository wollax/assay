//! Manifest generation from milestone chunks or all specs.
//!
//! Produces a [`RunManifest`] TOML that can be handed directly to Smelt
//! or to the local `run_manifest` pipeline. Two source modes:
//!
//! - **Milestone**: reads chunks from a named milestone, emitting
//!   `depends_on` from `ChunkRef.depends_on`.
//! - **AllSpecs**: scans all specs and produces fully-parallel sessions.
//!
//! See D009, D011, D012 for design decisions.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

#[cfg(feature = "orchestrate")]
use assay_types::OrchestratorMode;
use assay_types::{ManifestSession, RunManifest};

use crate::error::{AssayError, Result};
use crate::milestone::milestone_load;
use crate::spec;

/// Where to source manifest sessions from.
#[derive(Debug, Clone)]
pub enum ManifestSource {
    /// Generate sessions from a named milestone's chunk list.
    Milestone(String),
    /// Generate sessions from all specs in `.assay/specs/`.
    AllSpecs,
}

/// Configuration for manifest generation.
#[derive(Debug, Clone)]
pub struct ManifestGenConfig {
    /// Path to the `.assay/` directory.
    pub assay_dir: PathBuf,
}

/// Generate a [`RunManifest`] from the given source.
///
/// - `Milestone(slug)`: loads the milestone, maps each `ChunkRef` to a
///   `ManifestSession` with `depends_on` carried through from the chunk.
/// - `AllSpecs`: scans all specs and produces parallel sessions (no deps).
///
/// # Errors
///
/// Returns an error if the milestone is not found, has no chunks, or if
/// no specs are found in the specs directory.
pub fn generate_manifest(
    source: ManifestSource,
    config: &ManifestGenConfig,
) -> Result<RunManifest> {
    let sessions = match source {
        ManifestSource::Milestone(ref slug) => {
            let milestone = milestone_load(&config.assay_dir, slug)?;

            if milestone.chunks.is_empty() {
                return Err(AssayError::Io {
                    operation: format!("generating manifest: milestone '{}' has no chunks", slug),
                    path: config
                        .assay_dir
                        .join("milestones")
                        .join(format!("{slug}.toml")),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("milestone '{slug}' has no chunks"),
                    ),
                });
            }

            tracing::info!(
                source = "milestone",
                slug = %slug,
                chunks = milestone.chunks.len(),
                "generating manifest from milestone"
            );

            milestone
                .chunks
                .iter()
                .map(|chunk| ManifestSession {
                    spec: chunk.slug.clone(),
                    name: Some(chunk.slug.clone()),
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                    depends_on: chunk.depends_on.clone(),
                })
                .collect()
        }
        ManifestSource::AllSpecs => {
            let specs_dir = config.assay_dir.join("specs");
            let scan_result = spec::scan(&specs_dir)?;

            if scan_result.entries.is_empty() {
                return Err(AssayError::Io {
                    operation: "generating manifest: no specs found".to_string(),
                    path: specs_dir,
                    source: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "no specs found in specs directory",
                    ),
                });
            }

            tracing::info!(
                source = "all-specs",
                specs = scan_result.entries.len(),
                "generating manifest from all specs"
            );

            scan_result
                .entries
                .iter()
                .map(|entry| ManifestSession {
                    spec: entry.slug().to_string(),
                    name: Some(entry.slug().to_string()),
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                    depends_on: vec![],
                })
                .collect()
        }
    };

    let manifest = RunManifest {
        sessions,
        #[cfg(feature = "orchestrate")]
        mode: OrchestratorMode::Dag,
        ..Default::default()
    };

    Ok(manifest)
}

/// Write a [`RunManifest`] to disk atomically.
///
/// Serializes to TOML, writes via a temporary file in the parent directory,
/// then renames to the final path. A crash mid-write never leaves a partial
/// file.
///
/// # Errors
///
/// Returns an error if serialization fails, the parent directory does not
/// exist, or any I/O step fails.
pub fn write_manifest(manifest: &RunManifest, path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(manifest).map_err(|e| AssayError::Io {
        operation: "serializing manifest TOML".to_string(),
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
    })?;

    let parent = path.parent().unwrap_or(Path::new("."));

    let mut tmpfile = NamedTempFile::new_in(parent)
        .map_err(|e| AssayError::io("creating temp file for manifest", parent, e))?;

    tmpfile
        .write_all(content.as_bytes())
        .map_err(|e| AssayError::io("writing manifest", path, e))?;

    tmpfile
        .flush()
        .map_err(|e| AssayError::io("flushing manifest", path, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing manifest", path, e))?;

    tmpfile
        .persist(path)
        .map_err(|e| AssayError::io("persisting manifest", path, e.error))?;

    tracing::info!(
        path = %path.display(),
        sessions = manifest.sessions.len(),
        "wrote manifest"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: write a milestone TOML to the standard path, injecting
    /// required `created_at` and `updated_at` timestamps at the top level.
    fn write_milestone(assay_dir: &Path, slug: &str, chunks_content: &str) {
        let milestones_dir = assay_dir.join("milestones");
        std::fs::create_dir_all(&milestones_dir).unwrap();
        let path = milestones_dir.join(format!("{slug}.toml"));
        let now = chrono::Utc::now().to_rfc3339();
        // Split content: top-level fields must include timestamps BEFORE any [[chunks]]
        // Find where [[chunks]] starts (if any) and insert timestamps before it.
        let (header, rest) = if let Some(pos) = chunks_content.find("[[chunks]]") {
            (&chunks_content[..pos], &chunks_content[pos..])
        } else {
            (chunks_content, "")
        };
        let full_content =
            format!("{header}created_at = \"{now}\"\nupdated_at = \"{now}\"\n\n{rest}");
        std::fs::write(&path, full_content).unwrap();
    }

    /// Helper: write a minimal valid spec TOML file.
    fn write_spec(assay_dir: &Path, slug: &str) {
        let specs_dir = assay_dir.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();
        let path = specs_dir.join(format!("{slug}.toml"));
        let content = format!(
            r#"name = "{slug}"
description = "test spec"

[[criteria]]
name = "builds"
description = "project compiles"
cmd = "echo ok"
"#
        );
        std::fs::write(&path, content).unwrap();
    }

    #[test]
    fn test_generate_from_milestone_with_deps() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        write_milestone(
            assay_dir,
            "test",
            r#"
slug = "test"
name = "Test Milestone"

[[chunks]]
slug = "chunk-a"
order = 1

[[chunks]]
slug = "chunk-b"
order = 2
depends_on = ["chunk-a"]
"#,
        );

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let manifest =
            generate_manifest(ManifestSource::Milestone("test".into()), &config).unwrap();

        assert_eq!(manifest.sessions.len(), 2);

        let session_a = manifest
            .sessions
            .iter()
            .find(|s| s.spec == "chunk-a")
            .unwrap();
        assert!(session_a.depends_on.is_empty());
        assert_eq!(session_a.name, Some("chunk-a".to_string()));

        let session_b = manifest
            .sessions
            .iter()
            .find(|s| s.spec == "chunk-b")
            .unwrap();
        assert_eq!(session_b.depends_on, vec!["chunk-a"]);
    }

    #[test]
    fn test_generate_from_milestone_no_deps() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        write_milestone(
            assay_dir,
            "parallel",
            r#"
slug = "parallel"
name = "Parallel Milestone"

[[chunks]]
slug = "spec-a"
order = 1

[[chunks]]
slug = "spec-b"
order = 2
"#,
        );

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let manifest =
            generate_manifest(ManifestSource::Milestone("parallel".into()), &config).unwrap();

        assert_eq!(manifest.sessions.len(), 2);
        for session in &manifest.sessions {
            assert!(
                session.depends_on.is_empty(),
                "session {} should have no deps",
                session.spec
            );
        }
    }

    #[test]
    fn test_generate_from_milestone_empty_chunks_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        write_milestone(
            assay_dir,
            "empty",
            r#"
slug = "empty"
name = "Empty Milestone"
"#,
        );

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let result = generate_manifest(ManifestSource::Milestone("empty".into()), &config);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("has no chunks"),
            "error should mention 'has no chunks', got: {err}"
        );
    }

    #[test]
    fn test_generate_from_milestone_not_found_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let result = generate_manifest(ManifestSource::Milestone("nonexistent".into()), &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_generate_from_all_specs() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        write_spec(assay_dir, "auth");
        write_spec(assay_dir, "checkout");

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let manifest = generate_manifest(ManifestSource::AllSpecs, &config).unwrap();

        assert_eq!(manifest.sessions.len(), 2);
        for session in &manifest.sessions {
            assert!(session.depends_on.is_empty());
            assert_eq!(session.name.as_deref(), Some(session.spec.as_str()));
        }
    }

    #[test]
    fn test_generate_from_all_specs_empty_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        // Create empty specs dir
        std::fs::create_dir_all(assay_dir.join("specs")).unwrap();

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let result = generate_manifest(ManifestSource::AllSpecs, &config);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no specs found"),
            "error should mention 'no specs found', got: {err}"
        );
    }

    #[test]
    fn test_generate_roundtrips_through_validation() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        write_milestone(
            assay_dir,
            "roundtrip",
            r#"
slug = "roundtrip"
name = "Roundtrip Test"

[[chunks]]
slug = "step-a"
order = 1

[[chunks]]
slug = "step-b"
order = 2
depends_on = ["step-a"]
"#,
        );

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let manifest =
            generate_manifest(ManifestSource::Milestone("roundtrip".into()), &config).unwrap();

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&manifest).unwrap();

        // Parse back
        let parsed = crate::manifest::from_str(&toml_str).unwrap();

        // Validate
        let validation = crate::manifest::validate(&parsed);
        assert!(
            validation.is_ok(),
            "manifest should validate: {:?}",
            validation.err()
        );

        // Structural comparison
        assert_eq!(parsed.sessions.len(), 2);
        let session_b = parsed.sessions.iter().find(|s| s.spec == "step-b").unwrap();
        assert_eq!(session_b.depends_on, vec!["step-a"]);
    }

    #[test]
    fn test_write_manifest_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path();

        write_milestone(
            assay_dir,
            "write-test",
            r#"
slug = "write-test"
name = "Write Test"

[[chunks]]
slug = "task-1"
order = 1
"#,
        );

        let config = ManifestGenConfig {
            assay_dir: assay_dir.to_path_buf(),
        };
        let manifest =
            generate_manifest(ManifestSource::Milestone("write-test".into()), &config).unwrap();

        let output_path = tmp.path().join("output.toml");
        write_manifest(&manifest, &output_path).unwrap();

        // Read back and verify
        assert!(output_path.exists(), "manifest file should exist");
        let content = std::fs::read_to_string(&output_path).unwrap();
        let parsed: RunManifest = toml::from_str(&content).unwrap();
        assert_eq!(parsed.sessions.len(), 1);
        assert_eq!(parsed.sessions[0].spec, "task-1");
    }
}
