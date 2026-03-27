---
estimated_steps: 7
estimated_files: 2
---

# T02: Add backend field to GuardDaemon and wire try_save_checkpoint routing

**Slice:** S01 — GuardDaemon backend plumbing and contract tests
**Milestone:** M012

## Description

Core implementation task: add `Arc<dyn StateBackend>` field to `GuardDaemon` behind `#[cfg(feature = "orchestrate")]`, provide dual constructor signatures, route `try_save_checkpoint` through the backend when capability is present, update `start_guard()` public API, and fix `make_daemon()` test helper. This task turns the T01 red-state contract tests green.

## Steps

1. In `daemon.rs`, add feature-gated imports at the top of the file: `#[cfg(feature = "orchestrate")] use std::sync::Arc;` and `#[cfg(feature = "orchestrate")] use crate::state_backend::StateBackend;`
2. Add `#[cfg(feature = "orchestrate")] backend: Arc<dyn StateBackend>` field to the `GuardDaemon` struct.
3. Provide two `new()` implementations via `cfg`:
   - `#[cfg(feature = "orchestrate")] pub fn new(session_path, assay_dir, project_dir, config, backend: Arc<dyn StateBackend>) -> Self` — includes backend field
   - `#[cfg(not(feature = "orchestrate"))] pub fn new(session_path, assay_dir, project_dir, config) -> Self` — today's signature unchanged
4. Modify `try_save_checkpoint()` to route through backend when `orchestrate` feature is on:
   - Wrap the routing logic inside `#[cfg(feature = "orchestrate")]` block
   - Per D167, capture `let supports_checkpoints = self.backend.capabilities().supports_checkpoints;` once
   - If `supports_checkpoints` is true: after extracting `checkpoint` via `extract_team_state`, call `self.backend.save_checkpoint_summary(&self.assay_dir, &checkpoint)` with `info!("[guard] Checkpoint saved via backend")` on success and `warn!("[guard] Backend checkpoint save failed: {e}")` on error
   - If `supports_checkpoints` is false: fall through to existing `crate::checkpoint::save_checkpoint()` call
   - The `#[cfg(not(feature = "orchestrate"))]` path keeps today's code exactly as-is
5. In `mod.rs`, update `start_guard()` with dual signatures:
   - `#[cfg(feature = "orchestrate")] pub async fn start_guard(session_path, assay_dir, project_dir, config, backend: Arc<dyn StateBackend>)` — passes backend to `GuardDaemon::new`
   - `#[cfg(not(feature = "orchestrate"))] pub async fn start_guard(session_path, assay_dir, project_dir, config)` — today's signature
   - Add necessary feature-gated imports in `mod.rs`
6. Update `make_daemon()` test helper in `daemon.rs`:
   - `#[cfg(feature = "orchestrate")]` version passes `Arc::new(crate::state_backend::NoopBackend)` as the backend
   - `#[cfg(not(feature = "orchestrate"))]` version keeps today's signature
   - Update `guard_daemon_new_creates_valid_struct` test similarly
7. Run `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` to confirm all 9 existing tests + 2 contract tests pass. Run `cargo test -p assay-core -- guard::daemon::tests` (without orchestrate) to confirm non-orchestrate build passes.

## Must-Haves

- [ ] `GuardDaemon` has `#[cfg(feature = "orchestrate")] backend: Arc<dyn StateBackend>` field
- [ ] Two `new()` signatures via `cfg` — orchestrate (with backend) and non-orchestrate (without)
- [ ] `try_save_checkpoint()` routes through `backend.save_checkpoint_summary()` when orchestrate+supports_checkpoints
- [ ] `try_save_checkpoint()` uses existing `save_checkpoint()` when no orchestrate or capability false
- [ ] `start_guard()` has dual signatures in `mod.rs`
- [ ] `make_daemon()` updated for both feature states
- [ ] All 9 existing daemon tests pass under both feature flags
- [ ] Both contract tests from T01 pass (green state)

## Verification

- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — all tests pass including contract tests
- `cargo test -p assay-core -- guard::daemon::tests` — non-orchestrate build compiles and passes
- `cargo check -p assay-harness` — confirms assay-harness (no orchestrate feature) still compiles
- `cargo check -p assay-tui` — confirms assay-tui (no orchestrate feature) still compiles

## Observability Impact

- Signals added/changed: `info!("[guard] Checkpoint saved via backend")` when backend path taken; `warn!("[guard] Backend checkpoint save failed: {e}")` on backend error — allows distinguishing backend vs local checkpoint saves in logs
- How a future agent inspects this: grep guard daemon logs for "via backend" to confirm routing; absence means local path was used
- Failure state exposed: Backend save failure is logged as warn (non-fatal) — daemon continues operating

## Inputs

- `crates/assay-core/src/guard/daemon.rs` — T01 output: SpyBackend + contract tests in test module
- `crates/assay-core/src/guard/mod.rs` — current `start_guard()` with 4 params
- `crates/assay-core/src/state_backend.rs` — `StateBackend`, `NoopBackend`, `Arc<dyn StateBackend>` pattern

## Expected Output

- `crates/assay-core/src/guard/daemon.rs` — `GuardDaemon` with backend field, dual constructors, routing in `try_save_checkpoint`, updated test helpers, all tests green
- `crates/assay-core/src/guard/mod.rs` — `start_guard()` with dual signatures passing backend through
