---
estimated_steps: 4
estimated_files: 2
---

# T03: Wire CLI handle_guard_start and run just ready

**Slice:** S01 — GuardDaemon backend plumbing and contract tests
**Milestone:** M012

## Description

Final wiring task: update `handle_guard_start` in assay-cli to pass `Arc::new(LocalFsBackend::new(assay.clone()))` as the backend parameter to `start_guard()`. This preserves existing behavior for all local users (LocalFsBackend delegates to `crate::checkpoint::save_checkpoint` via its `save_checkpoint_summary` implementation). Run `just ready` to confirm the full workspace is green with 1526+ tests and zero regressions. This task validates R080.

## Steps

1. In `crates/assay-cli/src/commands/context.rs`, add imports in the `#[cfg(unix)]` `handle_guard_start` function scope: `use std::sync::Arc;` and `use assay_core::LocalFsBackend;` (both already available since assay-cli depends on assay-core with orchestrate feature).
2. Before the `rt.block_on(assay_core::guard::start_guard(...))` call, construct the backend: `let backend = Arc::new(LocalFsBackend::new(assay.clone()));`. Pass `backend` as the fifth argument to `start_guard()`.
3. Verify the `#[cfg(not(unix))]` stub `handle_guard_start` doesn't need changes (it returns an error without calling `start_guard`).
4. Run `just ready` — confirm full workspace green with 1526+ tests, zero failures.

## Must-Haves

- [ ] `handle_guard_start` passes `Arc::new(LocalFsBackend::new(assay.clone()))` to `start_guard()`
- [ ] No behavior change for existing local users — `LocalFsBackend.save_checkpoint_summary` delegates to `crate::checkpoint::persistence::save_checkpoint()`, same as today's direct call
- [ ] `just ready` green with 1526+ tests, zero failures
- [ ] R080 validated: checkpoint persistence routed through StateBackend

## Verification

- `cargo build -p assay-cli` — compiles with new backend parameter
- `just ready` — full workspace green with 1526+ tests

## Observability Impact

- Signals added/changed: None beyond what T02 added — CLI just passes the backend through
- How a future agent inspects this: run `assay context guard start` with a real session; grep logs for checkpoint messages
- Failure state exposed: None new — existing error handling preserved

## Inputs

- `crates/assay-cli/src/commands/context.rs` — current `handle_guard_start` calling `start_guard` with 4 params
- T02 output: `start_guard()` now accepts 5 params (with backend) behind `#[cfg(feature = "orchestrate")]`

## Expected Output

- `crates/assay-cli/src/commands/context.rs` — `handle_guard_start` passes `Arc::new(LocalFsBackend::new(assay.clone()))` to `start_guard()`
- `just ready` green — R080 validated, M012 milestone complete
