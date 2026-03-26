# S01: StateBackend trait and CapabilitySet

**Goal:** Define the `StateBackend` trait, `CapabilitySet` flags struct, `StateBackendConfig` enum, and `LocalFsBackend` skeleton — locking the API surface S02 will wire into the orchestrator. All contract tests must pass. `just ready` stays green (no orchestrator wiring in this slice).
**Demo:** `cargo test -p assay-types -p assay-core` passes with new `state_backend_config_schema_snapshot` test, `CapabilitySet` construction tests, `StateBackend` object-safety compile check, and `LocalFsBackend` stub instantiation test — all green alongside existing 1466 tests.

## Must-Haves

- `pub trait StateBackend: Send + Sync` in `assay_core::state_backend` with exactly 7 sync methods matching the S01→S02 boundary map signatures
- `fn _assert_object_safe(_: Box<dyn StateBackend>) {}` compile-time object-safety guard present in the module
- `CapabilitySet { supports_messaging, supports_gossip_manifest, supports_annotations, supports_checkpoints }` with `CapabilitySet::all()` and `CapabilitySet::none()` constructors
- `LocalFsBackend { assay_dir: PathBuf }` implements `StateBackend`; all methods return `Ok(())` or sensible defaults (stubs for S02); `capabilities()` returns `CapabilitySet::all()`
- `StateBackendConfig` enum in `assay-types` with `LocalFs` and `Custom { name: String, config: serde_json::Value }` variants; `serde(rename_all = "snake_case")` on the enum; inventory-registered schema
- `state_backend_config_schema_snapshot` test in `crates/assay-types/tests/schema_snapshots.rs` passes and is locked by `cargo insta review`
- All existing 1466 tests continue to pass; `just ready` green

## Proof Level

- This slice proves: contract
- Real runtime required: no
- Human/UAT required: no

## Verification

- `cargo test -p assay-types` — passes including `state_backend_config_schema_snapshot`
- `cargo test -p assay-core state_backend` — passes all new contract tests
- `cargo test --workspace` — passes all 1466+ tests (no regressions)
- `just ready` — fmt + lint + test + deny all green
- Snapshot locked: `cargo insta review` shows no pending snapshots after T02

## Observability / Diagnostics

- Runtime signals: None (S01 is type definitions and stubs — no runtime behaviour yet)
- Inspection surfaces: `capabilities()` method returns `CapabilitySet` — callers can log which capabilities a backend advertises; structured error from `AssayError` on any method failure (S02 onwards)
- Failure visibility: `StateBackend` trait methods return `Result<_, AssayError>` — all failures carry typed context via the existing error hierarchy
- Redaction constraints: None — no secrets or PII in this module

## Integration Closure

- Upstream surfaces consumed: `assay_types::orchestrate::OrchestratorStatus`, `assay_types::checkpoint::TeamCheckpoint`, `assay_core::AssayError` (all pre-existing)
- New wiring introduced in this slice: `assay_core::state_backend` module re-exported from `assay_core::lib`; `assay_types::state_backend` module with `StateBackendConfig` re-exported from `assay_types::lib`
- What remains before the milestone is truly usable end-to-end: S02 wires `LocalFsBackend` into `OrchestratorConfig`, routes all persistence calls through the trait, and adds `RunManifest.state_backend` field

## Tasks

- [ ] **T01: Define StateBackendConfig enum in assay-types with schema snapshot** `est:45m`
  - Why: `StateBackendConfig` belongs in `assay-types` (persisted type / config contract); it must be registered in the schema registry and have a locked snapshot before S02 adds it to `RunManifest`. This task establishes the type contract independently of the trait.
  - Files: `crates/assay-types/src/state_backend.rs` (new), `crates/assay-types/src/lib.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do:
    1. Create `crates/assay-types/src/state_backend.rs` with `StateBackendConfig` enum: variants `LocalFs` and `Custom { name: String, config: serde_json::Value }`. Derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`. Add `#[serde(rename_all = "snake_case")]` on the enum. Add `inventory::submit!` block registering `"state-backend-config"` schema entry.
    2. Add `pub mod state_backend;` and `pub use state_backend::StateBackendConfig;` to `crates/assay-types/src/lib.rs`.
    3. Add `state_backend_config_schema_snapshot` test function to `crates/assay-types/tests/schema_snapshots.rs` asserting `assert_json_snapshot!("state-backend-config-schema", schema.to_value())`.
    4. Run `cargo test -p assay-types state_backend_config_schema_snapshot -- --force-update-snapshots` to generate the snapshot, then `cargo insta review` to accept it.
    5. Verify `cargo test -p assay-types` passes (all existing + new snapshot test).
  - Verify: `cargo test -p assay-types` passes; `state_backend_config_schema_snapshot` test present and green; no pending insta snapshots
  - Done when: `cargo test -p assay-types` all green; schema snapshot file exists at `crates/assay-types/src/snapshots/state-backend-config-schema.snap`

- [ ] **T02: Define StateBackend trait, CapabilitySet, and LocalFsBackend skeleton in assay-core** `est:1h`
  - Why: The trait and skeleton are the core deliverable of this slice — they lock the API surface S02 will wire. Object-safety must be proven at compile time. `LocalFsBackend` must instantiate so downstream code can write against a concrete type. Contract tests prove the trait and flags struct behave correctly.
  - Files: `crates/assay-core/src/state_backend.rs` (new), `crates/assay-core/src/lib.rs`, `crates/assay-core/tests/state_backend.rs` (new)
  - Do:
    1. Create `crates/assay-core/src/state_backend.rs` with:
       - `pub trait StateBackend: Send + Sync` with exactly these 7 sync methods (all return `crate::Result<_>`):
         - `fn capabilities(&self) -> CapabilitySet`
         - `fn push_session_event(&self, assay_dir: &Path, status: &OrchestratorStatus) -> crate::Result<()>`
         - `fn read_run_state(&self, run_dir: &Path) -> crate::Result<Option<OrchestratorStatus>>`
         - `fn send_message(&self, inbox_path: &Path, name: &str, contents: &[u8]) -> crate::Result<()>`
         - `fn poll_inbox(&self, inbox_path: &Path) -> crate::Result<Vec<(String, Vec<u8>)>>`
         - `fn annotate_run(&self, run_dir: &Path, manifest_path: &str) -> crate::Result<()>`
         - `fn save_checkpoint_summary(&self, assay_dir: &Path, checkpoint: &TeamCheckpoint) -> crate::Result<()>`
       - Object-safety compile guard: `fn _assert_object_safe(_: Box<dyn StateBackend>) {}` (private, no `#[allow(dead_code)]` needed — it's a compile-time proof, not dead code in the usual sense)
       - `CapabilitySet` struct with `pub supports_messaging: bool`, `pub supports_gossip_manifest: bool`, `pub supports_annotations: bool`, `pub supports_checkpoints: bool`. Derive `Debug, Clone, Copy, PartialEq`. Add `impl CapabilitySet { pub fn all() -> Self { ... } pub fn none() -> Self { ... } }`.
       - `LocalFsBackend { pub assay_dir: PathBuf }`. Add `impl StateBackend for LocalFsBackend`. All methods: stub bodies — `push_session_event` returns `Ok(())`, `read_run_state` returns `Ok(None)`, `send_message` returns `Ok(())`, `poll_inbox` returns `Ok(vec![])`, `annotate_run` returns `Ok(())`, `save_checkpoint_summary` returns `Ok(())`. `capabilities()` returns `CapabilitySet::all()`.
    2. Add necessary imports at top of file: `use std::path::{Path, PathBuf};`, `use assay_types::{OrchestratorStatus, TeamCheckpoint};`.
    3. Add `pub mod state_backend;` and `pub use state_backend::{CapabilitySet, LocalFsBackend, StateBackend};` to `crates/assay-core/src/lib.rs`.
    4. Create `crates/assay-core/tests/state_backend.rs` with contract tests:
       - `test_capability_set_all()` — asserts all four flags are true
       - `test_capability_set_none()` — asserts all four flags are false
       - `test_local_fs_backend_capabilities_all_true()` — instantiates `LocalFsBackend` and checks `capabilities()` returns all-true
       - `test_local_fs_backend_as_trait_object()` — constructs `Box<dyn StateBackend>` from `LocalFsBackend`, calls `capabilities()`, asserts `supports_messaging` is true
       - `test_local_fs_backend_push_session_event_noop()` — calls `push_session_event` with a temp dir and a minimal `OrchestratorStatus` (run_id, phase, failure_policy, sessions, started_at), asserts `Ok(())`
       - `test_local_fs_backend_read_run_state_returns_none()` — calls `read_run_state` with a temp dir, asserts `Ok(None)`
    5. Run `cargo test -p assay-core state_backend` to verify all contract tests pass.
    6. Run `cargo test --workspace` to confirm all 1466+ tests still pass.
  - Verify: `cargo test -p assay-core state_backend` passes (6 new tests); `cargo test --workspace` passes with ≥1466 tests total; code compiles without warnings
  - Done when: 6+ contract tests in `crates/assay-core/tests/state_backend.rs` pass; `Box<dyn StateBackend>` construction is compiler-proven via the `_assert_object_safe` guard; no regressions

## Files Likely Touched

- `crates/assay-types/src/state_backend.rs` (new)
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-types/src/snapshots/state-backend-config-schema.snap` (new, generated)
- `crates/assay-core/src/state_backend.rs` (new)
- `crates/assay-core/src/lib.rs`
- `crates/assay-core/tests/state_backend.rs` (new)
