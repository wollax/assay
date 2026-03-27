---
estimated_steps: 4
estimated_files: 1
---

# T01: Create SpyBackend and red-state contract tests

**Slice:** S01 — GuardDaemon backend plumbing and contract tests
**Milestone:** M012

## Description

Test-first task: create the `SpyBackend` test helper and two contract tests in `daemon.rs` that define the expected routing behavior. The tests will fail to compile because `GuardDaemon::new()` doesn't accept a backend parameter yet — this red state is intentional and will be resolved in T02.

`SpyBackend` implements `StateBackend` with `supports_checkpoints = true` and records all `save_checkpoint_summary` calls via `Arc<Mutex<Vec<TeamCheckpoint>>>`. This is the verification instrument for the entire slice.

## Steps

1. In `crates/assay-core/src/guard/daemon.rs` test module, add feature-gated imports: `#[cfg(feature = "orchestrate")] use std::sync::{Arc, Mutex};` and `#[cfg(feature = "orchestrate")] use crate::state_backend::*;` and `#[cfg(feature = "orchestrate")] use assay_types::TeamCheckpoint;`
2. Add `SpyBackend` struct behind `#[cfg(feature = "orchestrate")]` in the test module: has a `calls: Arc<Mutex<Vec<TeamCheckpoint>>>` field and a `new() -> (Self, Arc<Mutex<Vec<TeamCheckpoint>>>)` constructor that returns both the backend and a handle to the recording vec. Implements `StateBackend` with `capabilities()` returning `CapabilitySet { supports_checkpoints: true, ..CapabilitySet::none() }` (only checkpoints enabled). `save_checkpoint_summary` clones the checkpoint and pushes it to `calls`. All other methods return `Ok(())` / `Ok(None)` / `Ok(vec![])`.
3. Add `#[test] #[cfg(feature = "orchestrate")] fn contract_backend_called_when_supports_checkpoints()` — creates a `SpyBackend`, constructs a `GuardDaemon` with it (this line will fail to compile in red state), creates a minimal project dir with `.git` and file structure needed for `extract_team_state`, calls `daemon.try_save_checkpoint("test")`, asserts `calls.lock().unwrap().len() == 1`.
4. Add `#[test] #[cfg(feature = "orchestrate")] fn contract_backend_not_called_when_no_checkpoint_capability()` — creates `NoopBackend` (supports_checkpoints=false), constructs a `GuardDaemon` with it, calls `try_save_checkpoint("test")`, asserts the local `save_checkpoint` path runs (verify by checking `.assay/checkpoints/` directory for a file, or that no panic occurred).

## Must-Haves

- [ ] `SpyBackend` implements all 7 `StateBackend` methods
- [ ] `SpyBackend::capabilities()` returns `supports_checkpoints: true`, all others false
- [ ] `SpyBackend::save_checkpoint_summary()` records the `TeamCheckpoint` clone in `Arc<Mutex<Vec<TeamCheckpoint>>>`
- [ ] Contract test `contract_backend_called_when_supports_checkpoints` exists with correct assertion
- [ ] Contract test `contract_backend_not_called_when_no_checkpoint_capability` exists with correct assertion
- [ ] All code is behind `#[cfg(feature = "orchestrate")]` / `#[cfg(test)]`

## Verification

- `SpyBackend` struct compiles in isolation: `cargo check -p assay-core --features orchestrate` (ignoring test compilation errors from missing `GuardDaemon::new` backend param — that's expected red state)
- Both test functions have the correct structure and assertions visible in the source

## Observability Impact

- Signals added/changed: None (test-only code)
- How a future agent inspects this: Read the test functions in `daemon.rs` to understand the contract
- Failure state exposed: None

## Inputs

- `crates/assay-core/src/guard/daemon.rs` — existing test module with `make_daemon()` helper and 9 tests
- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait, `NoopBackend`, `CapabilitySet`
- `crates/assay-types/src/checkpoint.rs` — `TeamCheckpoint` (derives Clone)

## Expected Output

- `crates/assay-core/src/guard/daemon.rs` — test module extended with `SpyBackend` + 2 contract test functions (red state — won't compile until T02)
