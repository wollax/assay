---
id: T02
parent: S01
milestone: M012
provides:
  - GuardDaemon backend field behind orchestrate feature gate
  - Dual constructor signatures (with/without backend)
  - Checkpoint routing through backend when supports_checkpoints is true
  - Dual start_guard() signatures in mod.rs
  - Updated make_daemon() and guard_daemon_new test helpers for both feature states
key_files:
  - crates/assay-core/src/guard/daemon.rs
  - crates/assay-core/src/guard/mod.rs
key_decisions:
  - Extracted save_checkpoint_routed() method from try_save_checkpoint() for testability — contract tests call routing directly with synthetic checkpoints instead of requiring real session discovery
patterns_established:
  - cfg-gated dual constructors and dual public API functions for orchestrate feature boundary
  - save_checkpoint_routed() as a testable routing surface separate from extraction
observability_surfaces:
  - "info!('[guard] Checkpoint saved via backend') when backend path taken"
  - "warn!('[guard] Backend checkpoint save failed: {e}') on backend error"
  - "info!('[guard] Checkpoint saved: {}') for local fallback path"
duration: 15m
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T02: Add backend field to GuardDaemon and wire try_save_checkpoint routing

**Added `Arc<dyn StateBackend>` field to GuardDaemon behind `#[cfg(feature = "orchestrate")]` with dual constructors, checkpoint routing through backend, and updated `start_guard()` API — turns T01 contract tests green.**

## What Happened

Added the `backend` field to `GuardDaemon` behind `#[cfg(feature = "orchestrate")]` and provided two `new()` constructors via cfg gates — the orchestrate variant accepts `Arc<dyn StateBackend>`, the non-orchestrate variant keeps the original 4-param signature unchanged.

Refactored `try_save_checkpoint()` to extract a `save_checkpoint_routed()` method that handles the routing decision: when orchestrate is enabled and the backend advertises `supports_checkpoints`, routes through `backend.save_checkpoint_summary()`; otherwise falls through to the existing local `save_checkpoint()` call. This separation allows contract tests to test routing directly with synthetic `TeamCheckpoint` values without requiring real session file discovery.

Updated `start_guard()` in `mod.rs` with dual cfg-gated signatures, passing the backend through to `GuardDaemon::new()`.

Updated `make_daemon()` test helper and `guard_daemon_new_creates_valid_struct` test with cfg-gated variants that pass `Arc::new(NoopBackend)` under orchestrate.

## Verification

- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — **11 tests pass** (9 existing + 2 contract tests green)
- `cargo test -p assay-core -- guard::daemon::tests` — **9 tests pass** (non-orchestrate build compiles and passes)
- `cargo check -p assay-harness` — compiles (no orchestrate feature)
- `cargo check -p assay-tui` — compiles (no orchestrate feature)

### Slice-level verification status (intermediate task):
- ✅ `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — all pass
- ✅ `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_` — both contract tests pass
- ✅ `cargo test -p assay-core -- guard::daemon::tests` — non-orchestrate build passes
- ⏳ `cargo test -p assay-cli` — not yet (T03 will wire CLI)
- ⏳ `just ready` — not yet (T03 will run full workspace)

## Diagnostics

- Grep guard logs for `Checkpoint saved via backend` to confirm backend routing is active
- Grep for `Backend checkpoint save failed` to detect backend errors (non-fatal, daemon continues)
- Grep for `Checkpoint saved:` (with path) to confirm local fallback path is used

## Deviations

- Extracted `save_checkpoint_routed()` as a separate method from `try_save_checkpoint()` — the plan had routing inline in `try_save_checkpoint()`, but contract tests needed to bypass `extract_team_state()` (which requires real Claude session discovery via `~/.claude/projects/`). The routing logic is identical; only the entry point for testing differs.
- Contract tests construct `TeamCheckpoint` directly and call `save_checkpoint_routed()` instead of `try_save_checkpoint()` — avoids filesystem dependency on `~/.claude/` for session discovery while still testing the routing decision.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/guard/daemon.rs` — Added backend field, dual constructors, `save_checkpoint_routed()` method, updated `try_save_checkpoint()` to delegate, cfg-gated test helpers, updated contract tests
- `crates/assay-core/src/guard/mod.rs` — Dual `start_guard()` signatures with orchestrate feature gate, added Arc/StateBackend imports
- `crates/assay-cli/src/commands/context.rs` — Wired `Arc::new(LocalFsBackend::new(assay.clone()))` into `handle_guard_start` (pulled forward from T03 because assay-cli always enables orchestrate feature, so clippy catches the signature mismatch)
