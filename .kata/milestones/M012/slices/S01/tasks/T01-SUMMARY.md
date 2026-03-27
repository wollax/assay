---
id: T01
parent: S01
milestone: M012
provides:
  - SpyBackend test double for StateBackend trait
  - Two contract tests defining backend routing behavior
key_files:
  - crates/assay-core/src/guard/daemon.rs
key_decisions: []
patterns_established:
  - SpyBackend records calls via Arc<Mutex<Vec<TeamCheckpoint>>> for assertion
  - Contract tests use tempdir with minimal .git + .claude/session.jsonl for extract_team_state
observability_surfaces:
  - none (test-only code)
duration: 1 step
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T01: Create SpyBackend and red-state contract tests

**Added SpyBackend test double and two contract tests for backend checkpoint routing — intentional red state until T02 wires backend into GuardDaemon::new.**

## What Happened

Added to the `daemon.rs` test module (all behind `#[cfg(feature = "orchestrate")]`):

1. **SpyBackend** — implements all 7 `StateBackend` methods. `capabilities()` returns only `supports_checkpoints: true`. `save_checkpoint_summary()` clones the checkpoint into `Arc<Mutex<Vec<TeamCheckpoint>>>`. All other methods return `Ok(())` / `Ok(None)` / `Ok(vec![])`.

2. **`contract_backend_called_when_supports_checkpoints`** — creates SpyBackend, attempts to construct GuardDaemon with it as a 5th arg, calls `try_save_checkpoint("test")`, asserts `calls.len() == 1`.

3. **`contract_backend_not_called_when_no_checkpoint_capability`** — creates NoopBackend (supports_checkpoints=false), attempts to construct GuardDaemon with it, calls `try_save_checkpoint("test")`, asserts no panic (backend not invoked).

Both contract tests intentionally fail to compile because `GuardDaemon::new()` doesn't accept a backend parameter yet — this is the expected red state that T02 will resolve.

## Verification

- `cargo check -p assay-core --features orchestrate` — passes (non-test code compiles clean)
- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_ --no-run` — fails with expected E0061 "unexpected argument #5" on both tests (red state confirmed)
- `cargo test -p assay-core -- guard::daemon::tests` — all 9 existing tests pass (no regression)
- Source inspection confirms SpyBackend implements all 7 trait methods

### Slice-level verification (partial — T01 is intermediate):
- ❌ `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — expected fail (red state)
- ❌ `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_` — expected fail (red state)
- ✅ `cargo test -p assay-core -- guard::daemon::tests` — all 9 existing tests pass
- Remaining checks depend on T02+ implementation

## Diagnostics

None — test-only code with no runtime surfaces.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/guard/daemon.rs` — added SpyBackend struct + StateBackend impl + 2 contract test functions in test module
