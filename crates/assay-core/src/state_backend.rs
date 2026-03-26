//! StateBackend trait, CapabilitySet flags struct, and LocalFsBackend skeleton.
//!
//! The `StateBackend` trait is the abstraction boundary between the orchestrator
//! and storage. All methods return `crate::Result<_>` so S02 implementations
//! carry full `AssayError` context on failure.
//!
//! S01 only defines the API surface; all `LocalFsBackend` method bodies are
//! intentional no-ops (stubs). Real implementations land in S02.

use std::path::{Path, PathBuf};

use assay_types::{OrchestratorStatus, TeamCheckpoint};

// ---------------------------------------------------------------------------
// CapabilitySet
// ---------------------------------------------------------------------------

/// Boolean flags advertising which optional capabilities a [`StateBackend`]
/// implementation supports.
///
/// Callers should check these flags before invoking operations that may be
/// unimplemented on some backends. This is the primary diagnostic surface for
/// capability mismatches.
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
/// All methods are object-safe. The compile guard [`_assert_object_safe`]
/// verifies this at compile time. Implementations must be `Send + Sync` so
/// the orchestrator can share them across async tasks.
pub trait StateBackend: Send + Sync {
    /// Return the capability flags for this backend.
    fn capabilities(&self) -> CapabilitySet;

    /// Persist an [`OrchestratorStatus`] event to the run directory.
    fn push_session_event(&self, run_dir: &Path, status: &OrchestratorStatus) -> crate::Result<()>;

    /// Load the latest [`OrchestratorStatus`] from the run directory.
    ///
    /// Returns `Ok(None)` when no state has been persisted yet.
    fn read_run_state(&self, run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>>;

    /// Deliver a message into an inbox path.
    fn send_message(&self, inbox_path: &Path, name: &str, contents: &[u8]) -> crate::Result<()>;

    /// Poll all pending messages from an inbox path.
    ///
    /// Returns a list of `(name, contents)` pairs.
    fn poll_inbox(&self, inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>>;

    /// Annotate a run with a gossip manifest path.
    fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> crate::Result<()>;

    /// Persist a [`TeamCheckpoint`] summary under the assay directory.
    fn save_checkpoint_summary(
        &self,
        assay_dir: &Path,
        checkpoint: &TeamCheckpoint,
    ) -> crate::Result<()>;
}

/// Compile-time object-safety guard.
///
/// If any method on [`StateBackend`] violates object safety, this function
/// will fail to compile, surfacing the violation before S02 wires the trait
/// into `OrchestratorConfig`.
#[allow(dead_code)]
fn _assert_object_safe(_: Box<dyn StateBackend>) {}

// ---------------------------------------------------------------------------
// LocalFsBackend
// ---------------------------------------------------------------------------

/// Local filesystem backend skeleton.
///
/// All trait method bodies are intentional no-ops in S01. Real implementations
/// (file I/O, JSON serialization, atomic writes) land in S02.
pub struct LocalFsBackend {
    /// Root assay directory (`.assay/` or equivalent).
    pub assay_dir: PathBuf,
}

impl LocalFsBackend {
    /// Create a new `LocalFsBackend` rooted at `assay_dir`.
    pub fn new(assay_dir: PathBuf) -> Self {
        Self { assay_dir }
    }
}

impl StateBackend for LocalFsBackend {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::all()
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
