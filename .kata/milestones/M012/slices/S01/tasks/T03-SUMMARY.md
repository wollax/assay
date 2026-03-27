---
id: T03
parent: S01
milestone: M012
provides:
  - CLI handle_guard_start wired with Arc<LocalFsBackend> passed to start_guard()
  - Full workspace green — R080 validated
key_files:
  - crates/assay-cli/src/commands/context.rs
key_decisions: []
patterns_established: []
observability_surfaces:
  - None beyond T02 — CLI passes backend through without adding new surfaces
duration: 10m
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T03: Wire CLI handle_guard_start and run just ready

**CLI `handle_guard_start` passes `Arc::new(LocalFsBackend::new(assay.clone()))` to `start_guard()` — full workspace green with 1501 tests, zero failures, R080 validated.**

## What Happened

The CLI wiring was already completed as part of T02's implementation — `handle_guard_start` in `crates/assay-cli/src/commands/context.rs` already constructs `Arc::new(assay_core::state_backend::LocalFsBackend::new(assay.clone()))` and passes it as the fifth argument to `start_guard()`. The non-unix stub correctly returns an error without calling `start_guard`, needing no changes.

T03 verified all slice-level checks and confirmed the full workspace is green.

## Verification

- `cargo build -p assay-cli` — compiles cleanly
- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — 11 passed (9 existing + 2 contract tests)
- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_` — both contract tests pass
- `cargo test -p assay-core -- guard::daemon::tests` — 9 passed (non-orchestrate build)
- `cargo test -p assay-cli` — 52 passed
- `just ready` — 1501 tests passed, 0 failures, exit code 0

## Diagnostics

No new diagnostics added — CLI just passes the backend through. See T02 summary for runtime log signals (`Checkpoint saved via backend` vs `Checkpoint saved:`).

## Deviations

CLI wiring was already done in T02 (the implementation naturally included it when updating `start_guard()` signature). T03 focused on verification of all slice-level checks.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/context.rs` — already wired in T02; verified compiles and passes tests
