//! PeerUpdate signal types and delivery to Assay session inboxes.
//!
//! Signal delivery uses HTTP-first with filesystem fallback (D186).
//! Types mirror Assay's `assay-types::signal` schema exactly (D189).
//!
//! Inbox path convention (matches `assay-core/src/orchestrate/mesh.rs`):
//! `<repo>/.assay/orchestrator/<run_id>/mesh/<session_name>/inbox/<filename>`

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

/// Lightweight gate pass/fail/skip counts included in a [`PeerUpdate`].
///
/// Mirrors Assay's `assay_types::signal::GateSummary` exactly (D189).
/// Assay uses `deny_unknown_fields` — any field mismatch causes silent rejection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct GateSummary {
    /// Number of sessions/gates that passed.
    pub passed: u32,
    /// Number of sessions/gates that failed.
    pub failed: u32,
    /// Number of sessions/gates that were skipped.
    pub skipped: u32,
}

/// A signal delivered to a running Assay session when a peer job completes.
///
/// Carries context about what changed so the receiving session can adapt its
/// work without human intermediation.
///
/// Mirrors Assay's `assay_types::signal::PeerUpdate` exactly (D189).
/// Field names and types must match — Assay uses `deny_unknown_fields`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PeerUpdate {
    /// Job that completed the session triggering this signal.
    pub source_job: String,
    /// Name of the completed session within the source job.
    pub source_session: String,
    /// Files modified by the completed session.
    pub changed_files: Vec<String>,
    /// Lightweight gate result summary from the completed session.
    pub gate_summary: GateSummary,
    /// Git branch the source session worked on.
    pub branch: String,
}

/// Envelope for routing a [`PeerUpdate`] to a specific session.
///
/// Mirrors Assay's `assay_types::signal::SignalRequest` exactly (D189).
/// Posted to `POST /api/v1/signal` on the Assay signal endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignalRequest {
    /// Name of the target session that should receive this update.
    pub target_session: String,
    /// The peer update payload to deliver.
    pub update: PeerUpdate,
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

// ── HTTP signal delivery (D186) ────────────────────────────────────────

/// Build a `reqwest::Client` configured for signal delivery.
///
/// The client uses a 5-second timeout to avoid blocking on unresponsive
/// signal endpoints. One client should be shared across all deliveries
/// (connection pooling).
pub(crate) fn make_signal_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build signal HTTP client")
}

/// Deliver a [`SignalRequest`] to an Assay signal endpoint via HTTP POST.
///
/// Posts `signal_request` as JSON to `url`. If `token` is `Some`, adds
/// an `Authorization: Bearer <token>` header.
///
/// Returns the HTTP status code on success, or a `reqwest::Error` on
/// transport failure (timeout, connection refused, etc.).
pub(crate) async fn deliver_signal_http(
    client: &reqwest::Client,
    url: &str,
    signal_request: &SignalRequest,
    token: Option<&str>,
) -> Result<reqwest::StatusCode, reqwest::Error> {
    tracing::debug!(
        url = %url,
        target_session = %signal_request.target_session,
        has_token = token.is_some(),
        "deliver_signal_http: sending request"
    );

    let mut request = client.post(url).json(signal_request);

    if let Some(t) = token {
        request = request.header("Authorization", format!("Bearer {t}"));
    }

    let response = request.send().await?;
    let status = response.status();

    tracing::debug!(url = %url, status = %status, "deliver_signal_http: response received");

    Ok(status)
}
