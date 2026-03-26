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
}

impl CapabilitySet {
    /// All capabilities enabled.
    pub fn all() -> Self {
        Self {
            supports_messaging: true,
            supports_gossip_manifest: true,
            supports_annotations: true,
            supports_checkpoints: true,
        }
    }

    /// All capabilities disabled.
    pub fn none() -> Self {
        Self {
            supports_messaging: false,
            supports_gossip_manifest: false,
            supports_annotations: false,
            supports_checkpoints: false,
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
/// This is a **test helper** — it proves degradation paths in isolation
/// without requiring a real filesystem or remote backend.
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
        CapabilitySet::all()
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
}
