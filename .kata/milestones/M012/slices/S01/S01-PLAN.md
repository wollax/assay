# S01: GuardDaemon backend plumbing and contract tests

**Goal:** `GuardDaemon` accepts `Arc<dyn StateBackend>` at construction; `try_save_checkpoint` routes through it when `capabilities().supports_checkpoints` is true; falls back to local `save_checkpoint()` when false; `start_guard()` API updated; CLI wired with `LocalFsBackend`; contract tests prove routing with spy backend; `just ready` green with 1526+ tests.
**Demo:** Running `just ready` passes all tests including new contract tests that assert: (1) spy backend's `save_checkpoint_summary` is called with correct `TeamCheckpoint` when `supports_checkpoints = true`, (2) spy backend is NOT called when `supports_checkpoints = false` and local `save_checkpoint` runs instead. D175 and D176 appended to DECISIONS.md (already done in roadmap phase).

## Must-Haves

- `GuardDaemon` struct has `#[cfg(feature = "orchestrate")] backend: Arc<dyn StateBackend>` field
- `GuardDaemon::new()` has two signatures via `#[cfg(feature = "orchestrate")]` — one with backend, one without
- `try_save_checkpoint()` routes through `backend.save_checkpoint_summary()` when `orchestrate` feature on AND `backend.capabilities().supports_checkpoints` is true
- `try_save_checkpoint()` falls back to `crate::checkpoint::save_checkpoint()` when feature off or capability false — preserving today's behavior
- `start_guard()` gains `backend: Arc<dyn StateBackend>` parameter behind `#[cfg(feature = "orchestrate")]`
- CLI `handle_guard_start` passes `Arc::new(LocalFsBackend::new(assay.clone()))` — no behavior change for existing users
- `SpyBackend` test helper records `save_checkpoint_summary` calls via `Arc<Mutex<Vec<TeamCheckpoint>>>`
- Contract test: spy backend records call when `supports_checkpoints = true` → PASS
- Contract test: spy backend NOT called when `supports_checkpoints = false` → PASS
- All 9 existing daemon tests pass (updated `make_daemon` passes `Arc::new(NoopBackend)` when `orchestrate` on)
- `just ready` green with 1526+ tests, zero failures

## Proof Level

- This slice proves: contract + integration
- Real runtime required: no (contract tests use SpyBackend/NoopBackend, not real filesystem checkpoints or remote backends)
- Human/UAT required: yes — manual `assay context guard start` with a real session to confirm checkpoint file created; manual SshSyncBackend test to confirm remote file appears

## Verification

- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — all existing daemon tests pass with updated constructor
- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_` — new contract tests pass (spy backend called / not called)
- `cargo test -p assay-core -- guard::daemon::tests` — non-orchestrate build compiles and all tests pass (no backend field)
- `cargo test -p assay-cli` — CLI compiles with `Arc::new(LocalFsBackend::new(...))`
- `just ready` — full workspace green with 1526+ tests

## Observability / Diagnostics

- Runtime signals: `tracing::info!("[guard] Checkpoint saved via backend: ...")` when backend path taken; existing `info!("[guard] Checkpoint saved: ...")` for local path; `warn!` on backend save failure
- Inspection surfaces: guard daemon log output (stderr via tracing subscriber) — future agents grep for `Checkpoint saved via backend` vs `Checkpoint saved:` to verify routing
- Failure visibility: `warn!("[guard] Backend checkpoint save failed: {e}")` with the error from `save_checkpoint_summary`; non-fatal (daemon continues)
- Redaction constraints: none — checkpoint data is project metadata, no secrets

## Integration Closure

- Upstream surfaces consumed: `crate::state_backend::StateBackend` trait (M010/S01), `crate::state_backend::LocalFsBackend` (M010/S02), `crate::state_backend::NoopBackend` (M010/S01), `crate::checkpoint::extract_team_state()`, `crate::checkpoint::save_checkpoint()`
- New wiring introduced in this slice: `GuardDaemon.backend` field, `start_guard()` backend parameter, CLI `handle_guard_start` → `Arc::new(LocalFsBackend::new(...))`
- What remains before the milestone is truly usable end-to-end: nothing — M012 has only one slice (S01) and this slice completes the milestone

## Tasks

- [x] **T01: Create SpyBackend and red-state contract tests** `est:30m`
  - Why: Test-first — contract tests define the expected behavior before implementation changes. SpyBackend is the verification tool that records `save_checkpoint_summary` calls.
  - Files: `crates/assay-core/src/guard/daemon.rs` (test module additions)
  - Do: Add `SpyBackend` struct (implements `StateBackend`, records calls via `Arc<Mutex<Vec<TeamCheckpoint>>>`). Add `contract_backend_called_when_supports_checkpoints` test that constructs a daemon with SpyBackend (supports_checkpoints=true), calls `try_save_checkpoint`, asserts spy recorded the call. Add `contract_backend_not_called_when_no_checkpoint_capability` test with NoopBackend (supports_checkpoints=false), calls `try_save_checkpoint`, asserts local checkpoint path runs. Both tests will fail to compile because `GuardDaemon::new` doesn't accept backend yet — that's correct (red state).
  - Verify: `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_` fails to compile (expected red state)
  - Done when: Two contract test functions exist with correct assertions; SpyBackend compiles standalone

- [x] **T02: Add backend field to GuardDaemon and wire try_save_checkpoint routing** `est:45m`
  - Why: Core implementation — adds the `backend` field behind `#[cfg(feature = "orchestrate")]`, updates constructors, and routes `try_save_checkpoint` through the backend when capability is present.
  - Files: `crates/assay-core/src/guard/daemon.rs`, `crates/assay-core/src/guard/mod.rs`
  - Do: (1) Add `#[cfg(feature = "orchestrate")] backend: Arc<dyn StateBackend>` field to `GuardDaemon`. (2) Feature-gate `use crate::state_backend::*` imports. (3) Provide two `new()` signatures via `cfg` — one with backend param (orchestrate), one without. (4) In `try_save_checkpoint`, add conditional: when orchestrate feature on, check `self.backend.capabilities().supports_checkpoints`; if true, call `self.backend.save_checkpoint_summary(&self.assay_dir, &checkpoint)` with appropriate logging; if false, fall through to existing `save_checkpoint` call. (5) Update `start_guard()` in `mod.rs` to accept `backend: Arc<dyn StateBackend>` behind cfg, pass through to `GuardDaemon::new()`. (6) Update `make_daemon()` test helper to pass `Arc::new(NoopBackend)` when orchestrate on. (7) Per D167, capture `supports_checkpoints` bool once in `try_save_checkpoint`, not per call site.
  - Verify: `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — all 9 existing tests pass + 2 contract tests pass; `cargo test -p assay-core -- guard::daemon::tests` — non-orchestrate build passes
  - Done when: Contract tests green; all existing tests green; both feature-flag states compile

- [x] **T03: Wire CLI handle_guard_start and run just ready** `est:30m`
  - Why: Completes the call-site chain — CLI passes `Arc::new(LocalFsBackend::new(assay.clone()))` to `start_guard()`, preserving existing behavior. Validates full workspace green.
  - Files: `crates/assay-cli/src/commands/context.rs`
  - Do: (1) Add `use std::sync::Arc;` and `use assay_core::LocalFsBackend;` imports (already available since assay-cli enables orchestrate). (2) In `handle_guard_start`, construct `Arc::new(LocalFsBackend::new(assay.clone()))` and pass as the backend parameter to `start_guard()`. (3) Run `just ready` to confirm full workspace green with 1526+ tests. (4) Verify the non-unix stub `handle_guard_start` compiles (it doesn't call `start_guard`, so no change needed).
  - Verify: `just ready` — green with 1526+ tests, zero failures
  - Done when: `just ready` green; CLI compiles; no behavior change for existing users; R080 validated

## Files Likely Touched

- `crates/assay-core/src/guard/daemon.rs` — GuardDaemon struct, new(), try_save_checkpoint(), make_daemon(), contract tests, SpyBackend
- `crates/assay-core/src/guard/mod.rs` — start_guard() signature
- `crates/assay-cli/src/commands/context.rs` — handle_guard_start() backend wiring
