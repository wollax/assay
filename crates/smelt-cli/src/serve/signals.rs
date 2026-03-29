//! PeerUpdate signal types and filesystem-based delivery to Assay session inboxes.
//!
//! When a peer job completes a session, Smelt can deliver a `PeerUpdate` signal
//! to a target session's inbox directory on the host filesystem. The container
//! sees the file via the Docker/Compose bind-mount at `/workspace`.
//!
//! Inbox path convention (matches `assay-core/src/orchestrate/mesh.rs`):
//! `<repo>/.assay/orchestrator/<run_id>/mesh/<session_name>/inbox/<filename>`

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

/// A signal delivered to a running Assay session when a peer job completes.
///
/// Carries context about what changed so the receiving session can adapt its
/// work without human intermediation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PeerUpdate {
    /// Job that completed the session triggering this signal.
    pub source_job: String,
    /// Name of the completed session within the source job.
    pub source_session: String,
    /// Files modified by the completed session.
    pub changed_files: Vec<String>,
    /// Summary of quality gate results from the completed session.
    pub gate_summary: String,
    /// Result branch name where the completed session's work landed.
    pub branch_name: String,
}

/// Validate that a path component (session name or run ID) is safe for use in
/// filesystem paths.
///
/// A safe component is a single non-empty path segment with no directory
/// separators and no special names. This prevents directory traversal when
/// joining untrusted input onto a base path.
///
/// Rejects: empty strings, `.`, `..`, any value containing `/` or `\`.
/// Modelled on Assay's `LocalFsBackend::send_message()` validation
/// (`assay-core/src/state_backend.rs`).
fn validate_path_component(name: &str, field: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if name == "." || name == ".." {
        return Err(format!("{field} must not be '.' or '..', got: {name:?}"));
    }
    if name.contains('/') {
        return Err(format!("{field} must not contain '/', got: {name:?}"));
    }
    if name.contains('\\') {
        return Err(format!("{field} must not contain '\\', got: {name:?}"));
    }
    Ok(())
}

/// Validate that a session name is safe for use in filesystem paths.
///
/// See [`validate_path_component`] for the rules.
pub(crate) fn validate_session_name(name: &str) -> Result<(), String> {
    validate_path_component(name, "session name")
}

/// Validate that a run ID is safe for use in filesystem paths.
///
/// See [`validate_path_component`] for the rules.
pub(crate) fn validate_run_id(run_id: &str) -> Result<(), String> {
    validate_path_component(run_id, "run_id")
}

/// Deliver a `PeerUpdate` signal to a session's inbox directory.
///
/// Writes the signal as a JSON file using the atomic write pattern from Assay's
/// `LocalFsBackend::send_message()`: `NamedTempFile::new_in()` + `write_all()`
/// + `sync_all()` + `persist()`.
///
/// The inbox path follows Assay's mesh mode convention:
/// `<repo_path>/.assay/orchestrator/<run_id>/mesh/<session_name>/inbox/`
///
/// Creates the inbox directory tree if it doesn't exist.
///
/// Returns the path of the written file on success.
pub(crate) fn deliver_peer_update(
    repo_path: &Path,
    run_id: &str,
    session_name: &str,
    peer_update: &PeerUpdate,
) -> std::io::Result<PathBuf> {
    // Validate both path components to prevent directory traversal.
    validate_session_name(session_name)
        .map_err(|msg| std::io::Error::new(std::io::ErrorKind::InvalidInput, msg))?;
    validate_run_id(run_id)
        .map_err(|msg| std::io::Error::new(std::io::ErrorKind::InvalidInput, msg))?;

    // Build inbox path: <repo>/.assay/orchestrator/<run_id>/mesh/<session_name>/inbox/
    let inbox_dir = repo_path
        .join(".assay")
        .join("orchestrator")
        .join(run_id)
        .join("mesh")
        .join(session_name)
        .join("inbox");

    // Create directory tree if it doesn't exist.
    std::fs::create_dir_all(&inbox_dir)?;

    // Serialize PeerUpdate to JSON bytes.
    let json_bytes = serde_json::to_vec_pretty(peer_update)
        .map_err(|e| std::io::Error::other(format!("JSON serialization failed: {e}")))?;

    // Generate a unique filename. Use nanos for ordering plus uuid for
    // collision safety (system clock coarseness or concurrent calls).
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| std::io::Error::other(format!("system clock error: {e}")))?
        .as_nanos();
    let filename = format!(
        "peer_update_{}_{}.json",
        nanos,
        uuid::Uuid::new_v4().simple()
    );
    let final_path = inbox_dir.join(&filename);

    // Atomic write: NamedTempFile + write_all + sync_all + persist.
    let mut tmp = NamedTempFile::new_in(&inbox_dir)?;
    tmp.write_all(&json_bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(&final_path).map_err(|e| e.error)?;

    Ok(final_path)
}
