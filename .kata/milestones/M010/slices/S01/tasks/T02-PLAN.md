---
estimated_steps: 6
estimated_files: 3
---

# T02: Define StateBackend trait, CapabilitySet, and LocalFsBackend skeleton in assay-core

**Slice:** S01 — StateBackend trait and CapabilitySet
**Milestone:** M010

## Description

Create `assay_core::state_backend` module with the `StateBackend` trait (7 sync methods, object-safe), `CapabilitySet` flags struct, and `LocalFsBackend` skeleton implementation. All method bodies are stubs in S01 — real implementations come in S02. The object-safety compile guard proves `Box<dyn StateBackend>` works before S02 wires it into `OrchestratorConfig`. Contract tests cover construction, capabilities, and basic stub behavior.

## Steps

1. Create `crates/assay-core/src/state_backend.rs`:
   - Imports at top: `use std::path::{Path, PathBuf};`, `use assay_types::{OrchestratorStatus, TeamCheckpoint};`
   - Define `CapabilitySet`:
     ```rust
     #[derive(Debug, Clone, Copy, PartialEq)]
     pub struct CapabilitySet {
         pub supports_messaging: bool,
         pub supports_gossip_manifest: bool,
         pub supports_annotations: bool,
         pub supports_checkpoints: bool,
     }
     impl CapabilitySet {
         pub fn all() -> Self { Self { supports_messaging: true, supports_gossip_manifest: true, supports_annotations: true, supports_checkpoints: true } }
         pub fn none() -> Self { Self { supports_messaging: false, supports_gossip_manifest: false, supports_annotations: false, supports_checkpoints: false } }
     }
     ```
   - Define `StateBackend` trait:
     ```rust
     pub trait StateBackend: Send + Sync {
         fn capabilities(&self) -> CapabilitySet;
         fn push_session_event(&self, run_dir: &Path, status: &OrchestratorStatus) -> crate::Result<()>;
         fn read_run_state(&self, run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>>;
         fn send_message(&self, inbox_path: &Path, name: &str, contents: &[u8]) -> crate::Result<()>;
         fn poll_inbox(&self, inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>>;
         fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> crate::Result<()>;
         fn save_checkpoint_summary(&self, assay_dir: &Path, checkpoint: &TeamCheckpoint) -> crate::Result<()>;
     }
     ```
   - Add object-safety compile guard: `fn _assert_object_safe(_: Box<dyn StateBackend>) {}` (private function at module level — will trigger a compile error if any method violates object safety).
   - Define `LocalFsBackend`:
     ```rust
     pub struct LocalFsBackend {
         pub assay_dir: PathBuf,
     }
     impl LocalFsBackend {
         pub fn new(assay_dir: PathBuf) -> Self { Self { assay_dir } }
     }
     impl StateBackend for LocalFsBackend {
         fn capabilities(&self) -> CapabilitySet { CapabilitySet::all() }
         fn push_session_event(&self, _run_dir: &Path, _status: &OrchestratorStatus) -> crate::Result<()> { Ok(()) }
         fn read_run_state(&self, _run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>> { Ok(None) }
         fn send_message(&self, _inbox_path: &Path, _name: &str, _contents: &[u8]) -> crate::Result<()> { Ok(()) }
         fn poll_inbox(&self, _inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>> { Ok(vec![]) }
         fn annotate_run(&self, _run_dir: &Path, _manifest_path: &str) -> crate::Result<()> { Ok(()) }
         fn save_checkpoint_summary(&self, _assay_dir: &Path, _checkpoint: &TeamCheckpoint) -> crate::Result<()> { Ok(()) }
     }
     ```
   - Note: `OrchestratorStatus` is behind `#[cfg(feature = "orchestrate")]` in assay-types. The `state_backend` module in assay-core must also be gated: wrap the file contents in `#[cfg(feature = "orchestrate")]` or add the feature to the module declaration in lib.rs, since the trait uses `OrchestratorStatus`. Check which feature flag convention applies to the orchestrate module in `lib.rs` (`#[cfg(feature = "orchestrate")]`).

2. In `crates/assay-core/src/lib.rs`, add:
   ```rust
   #[cfg(feature = "orchestrate")]
   pub mod state_backend;
   #[cfg(feature = "orchestrate")]
   pub use state_backend::{CapabilitySet, LocalFsBackend, StateBackend};
   ```

3. Create `crates/assay-core/tests/state_backend.rs` with 6 contract tests:
   ```rust
   use assay_core::{CapabilitySet, LocalFsBackend, StateBackend};
   use assay_types::{OrchestratorPhase, OrchestratorStatus};
   use tempfile::tempdir;

   #[test]
   fn test_capability_set_all() {
       let caps = CapabilitySet::all();
       assert!(caps.supports_messaging);
       assert!(caps.supports_gossip_manifest);
       assert!(caps.supports_annotations);
       assert!(caps.supports_checkpoints);
   }

   #[test]
   fn test_capability_set_none() {
       let caps = CapabilitySet::none();
       assert!(!caps.supports_messaging);
       assert!(!caps.supports_gossip_manifest);
       assert!(!caps.supports_annotations);
       assert!(!caps.supports_checkpoints);
   }

   #[test]
   fn test_local_fs_backend_capabilities_all_true() {
       let dir = tempdir().unwrap();
       let backend = LocalFsBackend::new(dir.path().to_path_buf());
       let caps = backend.capabilities();
       assert!(caps.supports_messaging);
       assert!(caps.supports_gossip_manifest);
       assert!(caps.supports_annotations);
       assert!(caps.supports_checkpoints);
   }

   #[test]
   fn test_local_fs_backend_as_trait_object() {
       let dir = tempdir().unwrap();
       let backend: Box<dyn StateBackend> = Box::new(LocalFsBackend::new(dir.path().to_path_buf()));
       let caps = backend.capabilities();
       assert!(caps.supports_messaging);
   }

   #[test]
   fn test_local_fs_backend_push_session_event_noop() {
       let dir = tempdir().unwrap();
       let backend = LocalFsBackend::new(dir.path().to_path_buf());
       let status = OrchestratorStatus {
           run_id: "test-run".to_string(),
           phase: OrchestratorPhase::Running,
           failure_policy: assay_types::FailurePolicy::SkipDependents,
           sessions: vec![],
           started_at: chrono::Utc::now(),
           completed_at: None,
           mesh_status: None,
           gossip_status: None,
       };
       let result = backend.push_session_event(dir.path(), &status);
       assert!(result.is_ok());
   }

   #[test]
   fn test_local_fs_backend_read_run_state_returns_none() {
       let dir = tempdir().unwrap();
       let backend = LocalFsBackend::new(dir.path().to_path_buf());
       let result = backend.read_run_state(dir.path());
       assert!(matches!(result, Ok(None)));
   }
   ```
   Note: `OrchestratorStatus` fields must match the actual struct — inspect `crates/assay-types/src/orchestrate.rs` to get the exact fields before writing the test. Construct minimally (use `..Default::default()` if `OrchestratorStatus` has `Default`, or fill in required fields explicitly).

4. Run `cargo test -p assay-core --features orchestrate state_backend` to verify all 6 new contract tests pass.

5. Run `cargo test --workspace` to confirm all 1466+ tests still pass without regressions.

6. Run `just ready` to confirm fmt + lint + test + deny all pass.

## Must-Haves

- [ ] `crates/assay-core/src/state_backend.rs` exists with `StateBackend` trait (7 methods), `CapabilitySet` struct with `all()`/`none()`, `LocalFsBackend` struct implementing the trait, and `_assert_object_safe` compile guard
- [ ] The module is feature-gated behind `#[cfg(feature = "orchestrate")]` consistent with `assay_core::orchestrate`
- [ ] `CapabilitySet`, `LocalFsBackend`, and `StateBackend` are re-exported from `assay_core` lib root (behind the same feature gate)
- [ ] `crates/assay-core/tests/state_backend.rs` exists with 6 passing contract tests
- [ ] `Box<dyn StateBackend>` construction works without compiler error (proven by `test_local_fs_backend_as_trait_object`)
- [ ] `cargo test --workspace` passes all 1466+ tests (no regressions)
- [ ] `just ready` green

## Verification

- `cargo test -p assay-core --features orchestrate state_backend` — 6 new tests pass
- `cargo test --workspace` — ≥1466 total tests, all pass
- `just ready` — all checks pass
- `grep "_assert_object_safe" crates/assay-core/src/state_backend.rs` — confirms compile guard is present
- `cargo build -p assay-core --features orchestrate` — compiles without warnings

## Observability Impact

- Signals added/changed: `CapabilitySet` returned by `capabilities()` is the inspection surface — callers can log which capabilities are available before attempting operations. This is the diagnostic surface S02 and S03 will use.
- How a future agent inspects this: `backend.capabilities()` returns a `CapabilitySet` with boolean fields; a future agent can log this at backend construction time to diagnose capability mismatches.
- Failure state exposed: All trait methods return `Result<_, AssayError>` — any method failure in S02+ will carry the standard `AssayError` context (path + operation + source) for structured diagnosis.

## Inputs

- `crates/assay-types/src/orchestrate.rs` — `OrchestratorStatus` struct fields needed for test construction; confirm field names match exactly
- `crates/assay-types/src/checkpoint.rs` — `TeamCheckpoint` type signature for `save_checkpoint_summary`
- `crates/assay-core/src/lib.rs` — existing `#[cfg(feature = "orchestrate")] pub mod orchestrate` pattern to follow for feature gating
- T01 output: `StateBackendConfig` type (not used in T02 directly, but confirms assay-types module structure is compatible)

## Expected Output

- `crates/assay-core/src/state_backend.rs` — new file with trait, flags struct, skeleton implementation, and compile guard
- `crates/assay-core/src/lib.rs` — updated with feature-gated `pub mod state_backend` and re-exports
- `crates/assay-core/tests/state_backend.rs` — new test file with 6 passing contract tests
