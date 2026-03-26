//! StateBackend trait, CapabilitySet flags struct, and LocalFsBackend implementation.
//!
//! The `StateBackend` trait is the abstraction boundary between the orchestrator
//! and storage. All methods return `crate::Result<_>`, giving callers typed
//! failure context via `AssayError` rather than raw strings or panics.

use std::path::{Path, PathBuf};

use assay_types::{OrchestratorStatus, TeamCheckpoint};

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
/// Uses `Arc` (not `Box`) to match the ownership model S02 will use in
/// `OrchestratorConfig`, which derives `Clone` — `Box<dyn Trait>` is not
/// `Clone`, but `Arc<dyn Trait>` is. Both require identical object-safety rules.
// Intentionally never called; existence is the proof.
#[allow(dead_code)]
fn _assert_object_safe(_: std::sync::Arc<dyn StateBackend>) {}

// ---------------------------------------------------------------------------
// LocalFsBackend
// ---------------------------------------------------------------------------

/// Local filesystem backend.
///
/// Persists orchestrator state under `assay_dir` using atomic writes.
/// All method bodies are stubs until S02 wires the real implementations.
/// Any call to a stub emits a `tracing::warn!` so premature use is
/// immediately visible in test output and operator logs.
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

    fn push_session_event(
        &self,
        run_dir: &Path,
        _status: &OrchestratorStatus,
    ) -> crate::Result<()> {
        tracing::warn!(
            run_dir = %run_dir.display(),
            assay_dir = %self.assay_dir.display(),
            "LocalFsBackend::push_session_event is a stub — no state was persisted (S02)"
        );
        Ok(())
    }

    fn read_run_state(&self, _run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>> {
        Ok(None)
    }

    fn send_message(&self, inbox_path: &Path, name: &str, _contents: &[u8]) -> crate::Result<()> {
        tracing::warn!(
            inbox_path = %inbox_path.display(),
            message_name = %name,
            "LocalFsBackend::send_message is a stub — message was not delivered (S02)"
        );
        Ok(())
    }

    fn poll_inbox(&self, inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>> {
        tracing::warn!(
            inbox_path = %inbox_path.display(),
            "LocalFsBackend::poll_inbox is a stub — returning empty inbox (S02)"
        );
        Ok(vec![])
    }

    fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> crate::Result<()> {
        tracing::warn!(
            run_dir = %run_dir.display(),
            manifest_path = %manifest_path,
            "LocalFsBackend::annotate_run is a stub — annotation was not persisted (S02)"
        );
        Ok(())
    }

    fn save_checkpoint_summary(
        &self,
        assay_dir: &Path,
        _checkpoint: &TeamCheckpoint,
    ) -> crate::Result<()> {
        tracing::warn!(
            assay_dir = %assay_dir.display(),
            "LocalFsBackend::save_checkpoint_summary is a stub — checkpoint was not persisted (S02)"
        );
        Ok(())
    }
}
