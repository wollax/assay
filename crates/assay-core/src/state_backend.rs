//! StateBackend trait, CapabilitySet flags struct, and LocalFsBackend implementation.
//!
//! The `StateBackend` trait is the abstraction boundary between the orchestrator
//! and storage. All methods return `crate::Result<_>`, giving callers typed
//! failure context via `AssayError` rather than raw strings or panics.

use std::io::Write;
use std::path::{Path, PathBuf};

use assay_types::{OrchestratorStatus, TeamCheckpoint};
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// CapabilitySet
// ---------------------------------------------------------------------------

/// Boolean flags advertising which optional capabilities a [`StateBackend`]
/// implementation supports.
///
/// Callers should check these flags before invoking operations that may be
/// unsupported on some backends. When a capability is absent, the caller is
/// responsible for degrading gracefully (emit a `warn!`, skip the operation).
///
/// # Contract for backend implementors
/// When a method is called without the corresponding capability flag set,
/// implementations should return an error rather than silently no-op, so
/// callers that forget to check flags receive an actionable diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet {
    /// Whether the backend supports inter-session message passing.
    pub supports_messaging: bool,
    /// Whether the backend supports gossip manifest persistence.
    pub supports_gossip_manifest: bool,
    /// Whether the backend supports run annotations.
    pub supports_annotations: bool,
    /// Whether the backend supports team checkpoint persistence.
    pub supports_checkpoints: bool,
    /// Whether the backend supports external signal reception (e.g. HTTP endpoint).
    pub supports_signals: bool,
    /// Whether the backend supports peer registry (register/list/unregister peers).
    pub supports_peer_registry: bool,
}

impl CapabilitySet {
    /// All capabilities enabled.
    pub fn all() -> Self {
        Self {
            supports_messaging: true,
            supports_gossip_manifest: true,
            supports_annotations: true,
            supports_checkpoints: true,
            supports_signals: true,
            supports_peer_registry: true,
        }
    }

    /// All capabilities disabled.
    pub fn none() -> Self {
        Self {
            supports_messaging: false,
            supports_gossip_manifest: false,
            supports_annotations: false,
            supports_checkpoints: false,
            supports_signals: false,
            supports_peer_registry: false,
        }
    }
}

// ---------------------------------------------------------------------------
// StateBackend trait
// ---------------------------------------------------------------------------

/// Pluggable state persistence interface for the orchestrator.
///
/// All methods are object-safe (verified at compile time by `_assert_object_safe`).
/// Implementations must be `Send + Sync` so the orchestrator can share them
/// across worker threads via `thread::scope`.
///
/// # Capability contract
/// Before calling any optional method, callers should check [`CapabilitySet`]
/// via [`StateBackend::capabilities`]. Backend implementations where the
/// relevant capability flag is `false` should return an error — not silently
/// succeed — so callers that skip the capability check receive a diagnostic.
pub trait StateBackend: Send + Sync {
    /// Return the capability flags for this backend.
    fn capabilities(&self) -> CapabilitySet;

    /// Persist an [`OrchestratorStatus`] event to the run directory.
    ///
    /// # Contract
    /// Always supported (`capabilities()` has no corresponding flag).
    fn push_session_event(&self, run_dir: &Path, status: &OrchestratorStatus) -> crate::Result<()>;

    /// Load the latest [`OrchestratorStatus`] from the run directory.
    ///
    /// Returns `Ok(None)` when no state has been persisted yet.
    fn read_run_state(&self, run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>>;

    /// Deliver a message into an inbox path.
    ///
    /// `name` is used as a filename component. It must be non-empty and must
    /// not contain path separators or the components `.` or `..`.
    ///
    /// # Contract
    /// Implementations where `capabilities().supports_messaging` is `false`
    /// must return an error rather than silently no-op.
    fn send_message(&self, inbox_path: &Path, name: &str, contents: &[u8]) -> crate::Result<()>;

    /// Poll all pending messages from an inbox path.
    ///
    /// Returns a list of `(name, contents)` pairs.
    ///
    /// # Contract
    /// Implementations where `capabilities().supports_messaging` is `false`
    /// must return an error rather than silently no-op.
    fn poll_inbox(&self, inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>>;

    /// Annotate a run with a gossip manifest path.
    ///
    /// # Contract
    /// Implementations where `capabilities().supports_gossip_manifest` is `false`
    /// must return an error rather than silently no-op.
    fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> crate::Result<()>;

    /// Persist a [`TeamCheckpoint`] summary under the assay directory.
    ///
    /// # Contract
    /// Implementations where `capabilities().supports_checkpoints` is `false`
    /// must return an error rather than silently no-op.
    fn save_checkpoint_summary(
        &self,
        assay_dir: &Path,
        checkpoint: &TeamCheckpoint,
    ) -> crate::Result<()>;

    // ── Peer registry (optional, default no-op) ─────────────────────

    /// Register a peer instance that can receive signals.
    ///
    /// Default implementation is a no-op. Override when
    /// `capabilities().supports_peer_registry` is `true`.
    fn register_peer(&self, _peer: &assay_types::PeerInfo) -> crate::Result<()> {
        Ok(())
    }

    /// List all registered peer instances.
    ///
    /// Default implementation returns an empty list. Override when
    /// `capabilities().supports_peer_registry` is `true`.
    fn list_peers(&self) -> crate::Result<Vec<assay_types::PeerInfo>> {
        Ok(vec![])
    }

    /// Remove a peer by its identifier.
    ///
    /// Default implementation is a no-op. Override when
    /// `capabilities().supports_peer_registry` is `true`.
    fn unregister_peer(&self, _peer_id: &str) -> crate::Result<()> {
        Ok(())
    }
}

/// Compile-time object-safety guard.
///
/// If any method on [`StateBackend`] violates object safety (e.g. a generic
/// parameter or `-> Self` return), this function will fail to compile, catching
/// the violation at the definition site rather than at every construction point.
///
/// Uses `Arc` (not `Box`) to match the ownership model used in
/// `OrchestratorConfig`, which implements `Clone` via `Arc::clone` —
/// `Box<dyn Trait>` is not `Clone`, but `Arc<dyn Trait>` is.
// Intentionally never called; existence is the proof.
#[allow(dead_code)]
fn _assert_object_safe(_: std::sync::Arc<dyn StateBackend>) {}

// ---------------------------------------------------------------------------
// NoopBackend (test helper)
// ---------------------------------------------------------------------------

/// A no-op backend with all capabilities disabled.
///
/// All methods return `Ok(())` / `Ok(None)` / `Ok(vec![])` without performing
/// any I/O. `capabilities()` returns [`CapabilitySet::none()`] so callers that
/// check capabilities before calling optional methods will degrade gracefully
/// rather than invoking any method at all.
///
/// # Deliberate contract exception
///
/// [`StateBackend`]'s documented contract says implementations with a capability
/// flag `false` *should return an error* when that optional method is called.
/// `NoopBackend` intentionally breaks this — all methods succeed silently.
/// This is safe only because the orchestrator always checks [`CapabilitySet`]
/// before calling optional methods; `NoopBackend` is only valid when used with
/// a caller that honours that check (which all production paths do).
///
/// # Test helper
///
/// This is a **test helper** — it proves degradation paths in isolation without
/// requiring a real filesystem or remote backend. There is no enforcement
/// preventing production use, but do not use `NoopBackend` in production code:
/// all state writes will be silently discarded.
#[doc(hidden)]
pub struct NoopBackend;

impl StateBackend for NoopBackend {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::none()
    }

    fn push_session_event(
        &self,
        _run_dir: &Path,
        _status: &OrchestratorStatus,
    ) -> crate::Result<()> {
        Ok(())
    }

    fn read_run_state(&self, _run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>> {
        Ok(None)
    }

    fn send_message(&self, _inbox_path: &Path, _name: &str, _contents: &[u8]) -> crate::Result<()> {
        Ok(())
    }

    fn poll_inbox(&self, _inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>> {
        Ok(vec![])
    }

    fn annotate_run(&self, _run_dir: &Path, _manifest_path: &str) -> crate::Result<()> {
        Ok(())
    }

    fn save_checkpoint_summary(
        &self,
        _assay_dir: &Path,
        _checkpoint: &TeamCheckpoint,
    ) -> crate::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// LocalFsBackend
// ---------------------------------------------------------------------------

/// Local filesystem backend.
///
/// Persists orchestrator state under `assay_dir` using atomic writes
/// (tempfile-rename pattern) for crash safety. Reads tolerate missing
/// files gracefully, returning `Ok(None)` or `Ok(vec![])` as appropriate.
pub struct LocalFsBackend {
    /// Root assay directory (`.assay/` or equivalent).
    assay_dir: PathBuf,
}

impl LocalFsBackend {
    /// Create a new `LocalFsBackend` rooted at `assay_dir`.
    pub fn new(assay_dir: PathBuf) -> Self {
        Self { assay_dir }
    }

    /// Return the root assay directory this backend operates under.
    pub fn assay_dir(&self) -> &Path {
        &self.assay_dir
    }
}

impl StateBackend for LocalFsBackend {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_messaging: true,
            supports_gossip_manifest: true,
            supports_annotations: true,
            supports_checkpoints: true,
            supports_signals: false,
            supports_peer_registry: true,
        }
    }

    fn push_session_event(&self, run_dir: &Path, status: &OrchestratorStatus) -> crate::Result<()> {
        std::fs::create_dir_all(run_dir)
            .map_err(|e| crate::AssayError::io("creating run directory", run_dir, e))?;

        let final_path = run_dir.join("state.json");
        let json = serde_json::to_string_pretty(status).map_err(|e| {
            crate::AssayError::json("serializing orchestrator status", &final_path, e)
        })?;

        let mut tmpfile = NamedTempFile::new_in(run_dir).map_err(|e| {
            crate::AssayError::io("creating temp file for orchestrator state", run_dir, e)
        })?;

        tmpfile
            .write_all(json.as_bytes())
            .map_err(|e| crate::AssayError::io("writing orchestrator state", &final_path, e))?;

        tmpfile
            .as_file()
            .sync_all()
            .map_err(|e| crate::AssayError::io("syncing orchestrator state", &final_path, e))?;

        tmpfile.persist(&final_path).map_err(|e| {
            crate::AssayError::io("persisting orchestrator state", &final_path, e.error)
        })?;

        Ok(())
    }

    fn read_run_state(&self, run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>> {
        let state_path = run_dir.join("state.json");
        match std::fs::read_to_string(&state_path) {
            Ok(contents) => {
                let status = serde_json::from_str(&contents)
                    .map_err(|e| crate::AssayError::json("reading state.json", &state_path, e))?;
                Ok(Some(status))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(crate::AssayError::io("reading state.json", &state_path, e)),
        }
    }

    fn send_message(&self, inbox_path: &Path, name: &str, contents: &[u8]) -> crate::Result<()> {
        // Validate name to prevent path traversal.
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name == "."
            || name == ".."
        {
            return Err(crate::AssayError::io(
                "send_message name validation",
                inbox_path,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "name {name:?} must be non-empty and must not contain path separators or reserved components"
                    ),
                ),
            ));
        }

        std::fs::create_dir_all(inbox_path)
            .map_err(|e| crate::AssayError::io("creating inbox directory", inbox_path, e))?;

        let final_path = inbox_path.join(name);

        let mut tmpfile = NamedTempFile::new_in(inbox_path)
            .map_err(|e| crate::AssayError::io("creating temp file for message", inbox_path, e))?;

        tmpfile
            .write_all(contents)
            .map_err(|e| crate::AssayError::io("writing message", &final_path, e))?;

        tmpfile
            .as_file()
            .sync_all()
            .map_err(|e| crate::AssayError::io("syncing message", &final_path, e))?;

        tmpfile
            .persist(&final_path)
            .map_err(|e| crate::AssayError::io("persisting message", &final_path, e.error))?;

        Ok(())
    }

    fn poll_inbox(&self, inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>> {
        let entries = match std::fs::read_dir(inbox_path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => {
                return Err(crate::AssayError::io(
                    "reading inbox directory",
                    inbox_path,
                    e,
                ));
            }
        };

        // Phase 1: read all messages.
        let mut read_messages = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|e| crate::AssayError::io("reading inbox entry", inbox_path, e))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let contents = std::fs::read(&path)
                .map_err(|e| crate::AssayError::io("reading inbox message", &path, e))?;
            read_messages.push((path, name, contents));
        }

        // Phase 2: delete after all reads succeed, warn on individual failures.
        let mut messages = Vec::with_capacity(read_messages.len());
        for (path, name, contents) in read_messages {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "failed to remove inbox message after read — may be delivered twice"
                );
            }
            messages.push((name, contents));
        }
        Ok(messages)
    }

    fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> crate::Result<()> {
        std::fs::create_dir_all(run_dir)
            .map_err(|e| crate::AssayError::io("creating run directory", run_dir, e))?;

        let final_path = run_dir.join("gossip_manifest_path.txt");

        let mut tmpfile = NamedTempFile::new_in(run_dir)
            .map_err(|e| crate::AssayError::io("creating temp file for annotation", run_dir, e))?;

        tmpfile
            .write_all(manifest_path.as_bytes())
            .map_err(|e| crate::AssayError::io("writing annotation", &final_path, e))?;

        tmpfile
            .as_file()
            .sync_all()
            .map_err(|e| crate::AssayError::io("syncing annotation", &final_path, e))?;

        tmpfile
            .persist(&final_path)
            .map_err(|e| crate::AssayError::io("persisting annotation", &final_path, e.error))?;

        Ok(())
    }

    fn save_checkpoint_summary(
        &self,
        assay_dir: &Path,
        checkpoint: &TeamCheckpoint,
    ) -> crate::Result<()> {
        crate::checkpoint::persistence::save_checkpoint(assay_dir, checkpoint).map(|_| ())
    }

    // ── Peer registry ───────────────────────────────────────────────

    fn register_peer(&self, peer: &assay_types::PeerInfo) -> crate::Result<()> {
        let peers_path = self.assay_dir.join("peers.json");

        // Read existing peers (or start with empty list).
        let mut peers: Vec<assay_types::PeerInfo> = match std::fs::read_to_string(&peers_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|e| crate::AssayError::json("reading peers.json", &peers_path, e))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(e) => return Err(crate::AssayError::io("reading peers.json", &peers_path, e)),
        };

        // Upsert by peer_id.
        if let Some(existing) = peers.iter_mut().find(|p| p.peer_id == peer.peer_id) {
            *existing = peer.clone();
        } else {
            peers.push(peer.clone());
        }

        // Atomic write.
        std::fs::create_dir_all(&self.assay_dir)
            .map_err(|e| crate::AssayError::io("creating assay directory", &self.assay_dir, e))?;

        let json = serde_json::to_string_pretty(&peers)
            .map_err(|e| crate::AssayError::json("serializing peers.json", &peers_path, e))?;

        let mut tmpfile = NamedTempFile::new_in(&self.assay_dir).map_err(|e| {
            crate::AssayError::io("creating temp file for peers", &self.assay_dir, e)
        })?;
        tmpfile
            .write_all(json.as_bytes())
            .map_err(|e| crate::AssayError::io("writing peers.json", &peers_path, e))?;
        tmpfile
            .as_file()
            .sync_all()
            .map_err(|e| crate::AssayError::io("syncing peers.json", &peers_path, e))?;
        tmpfile
            .persist(&peers_path)
            .map_err(|e| crate::AssayError::io("persisting peers.json", &peers_path, e.error))?;

        Ok(())
    }

    fn list_peers(&self) -> crate::Result<Vec<assay_types::PeerInfo>> {
        let peers_path = self.assay_dir.join("peers.json");
        match std::fs::read_to_string(&peers_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|e| crate::AssayError::json("reading peers.json", &peers_path, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(crate::AssayError::io("reading peers.json", &peers_path, e)),
        }
    }

    fn unregister_peer(&self, peer_id: &str) -> crate::Result<()> {
        let peers_path = self.assay_dir.join("peers.json");

        let mut peers: Vec<assay_types::PeerInfo> = match std::fs::read_to_string(&peers_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|e| crate::AssayError::json("reading peers.json", &peers_path, e))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(crate::AssayError::io("reading peers.json", &peers_path, e)),
        };

        let original_len = peers.len();
        peers.retain(|p| p.peer_id != peer_id);

        // Only write back if something changed.
        if peers.len() == original_len {
            return Ok(());
        }

        let json = serde_json::to_string_pretty(&peers)
            .map_err(|e| crate::AssayError::json("serializing peers.json", &peers_path, e))?;

        let mut tmpfile = NamedTempFile::new_in(&self.assay_dir).map_err(|e| {
            crate::AssayError::io("creating temp file for peers", &self.assay_dir, e)
        })?;
        tmpfile
            .write_all(json.as_bytes())
            .map_err(|e| crate::AssayError::io("writing peers.json", &peers_path, e))?;
        tmpfile
            .as_file()
            .sync_all()
            .map_err(|e| crate::AssayError::io("syncing peers.json", &peers_path, e))?;
        tmpfile
            .persist(&peers_path)
            .map_err(|e| crate::AssayError::io("persisting peers.json", &peers_path, e.error))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod peer_tests {
    use super::*;
    use assay_types::PeerInfo;
    use chrono::Utc;

    fn make_peer(id: &str, url: &str) -> PeerInfo {
        PeerInfo {
            peer_id: id.to_string(),
            signal_url: url.to_string(),
            registered_at: Utc::now(),
        }
    }

    #[test]
    fn test_register_and_list_peers() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalFsBackend::new(tmp.path().to_path_buf());

        backend
            .register_peer(&make_peer("peer-1", "http://localhost:7432"))
            .unwrap();
        backend
            .register_peer(&make_peer("peer-2", "http://localhost:7433"))
            .unwrap();

        let peers = backend.list_peers().unwrap();
        assert_eq!(peers.len(), 2);
        assert!(peers.iter().any(|p| p.peer_id == "peer-1"));
        assert!(peers.iter().any(|p| p.peer_id == "peer-2"));
    }

    #[test]
    fn test_register_peer_upserts() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalFsBackend::new(tmp.path().to_path_buf());

        backend
            .register_peer(&make_peer("peer-1", "http://localhost:7432"))
            .unwrap();
        backend
            .register_peer(&make_peer("peer-1", "http://localhost:9999"))
            .unwrap();

        let peers = backend.list_peers().unwrap();
        assert_eq!(peers.len(), 1, "upsert should not duplicate");
        assert_eq!(peers[0].signal_url, "http://localhost:9999");
    }

    #[test]
    fn test_unregister_peer() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalFsBackend::new(tmp.path().to_path_buf());

        backend
            .register_peer(&make_peer("peer-1", "http://localhost:7432"))
            .unwrap();
        backend.unregister_peer("peer-1").unwrap();

        let peers = backend.list_peers().unwrap();
        assert!(peers.is_empty(), "should be empty after unregister");
    }

    #[test]
    fn test_unregister_nonexistent_peer() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalFsBackend::new(tmp.path().to_path_buf());

        // Should be Ok(()) even when there's no peers.json.
        backend.unregister_peer("nonexistent").unwrap();
    }

    #[test]
    fn test_list_peers_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let backend = LocalFsBackend::new(tmp.path().to_path_buf());

        let peers = backend.list_peers().unwrap();
        assert!(peers.is_empty(), "no peers.json should return empty vec");
    }
}
