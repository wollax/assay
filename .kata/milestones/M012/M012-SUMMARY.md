---
id: M012
provides:
  - GuardDaemon.backend field (Arc<dyn StateBackend>) set at construction under orchestrate feature gate
  - try_save_checkpoint routing — backend.save_checkpoint_summary() when supports_checkpoints=true; local save_checkpoint() fallback when false
  - save_checkpoint_routed() testable routing surface for contract tests
  - Dual GuardDaemon::new() constructors and dual start_guard() signatures (cfg-gated)
  - CLI handle_guard_start passes Arc::new(LocalFsBackend::new(&assay_dir)) — zero behavior change for local users
  - SpyBackend + two contract tests proving routing in both directions
  - R080 validated: checkpoint persistence routed through StateBackend
key_decisions:
  - D175: GuardDaemon accepts Arc<dyn StateBackend>; CLI defaults to LocalFsBackend
  - D176: save_checkpoint_summary called synchronously inside async GuardDaemon run loop (accepted risk)
  - cfg-gated dual constructors at orchestrate feature boundary to avoid type leakage
  - save_checkpoint_routed() extracted as testable surface to avoid real-session filesystem dependency in contract tests
patterns_established:
  - SpyBackend test double with Arc<Mutex<Vec<TeamCheckpoint>>> for assertion
  - Contract tests construct TeamCheckpoint directly; no filesystem dependency on ~/.claude/ session discovery
  - cfg-gated dual public API functions at orchestrate feature boundary
observability_surfaces:
  - "info!('[guard] Checkpoint saved via backend (trigger={trigger})') when backend path taken"
  - "warn!('[guard] Backend checkpoint save failed: {e}') on backend error (non-fatal)"
  - "info!('[guard] Checkpoint saved: {}') for local fallback path"
requirement_outcomes:
  - id: R080
    from_status: active
    to_status: validated
    proof: >
      GuardDaemon.backend: Arc<dyn StateBackend> field exists behind orchestrate feature gate.
      save_checkpoint_routed() routes through backend.save_checkpoint_summary() when
      supports_checkpoints=true (SpyBackend contract test passes). Falls back to local
      save_checkpoint() when false (second SpyBackend contract test passes).
      start_guard() signatures include Arc<dyn StateBackend> parameter under orchestrate gate;
      all call sites updated. CLI handle_guard_start passes Arc::new(LocalFsBackend::new(&assay_dir)).
      just ready green with 1529 tests passing, zero failures.
      D175 and D176 documented in DECISIONS.md.
duration: 1 session (3 tasks)
verification_result: passed
completed_at: 2026-03-27
---

# M012: Checkpoint Persistence on Remote Backends

**Routes `GuardDaemon` checkpoint writes through `Arc<dyn StateBackend>`, making `save_checkpoint_summary` a real production call site for the first time — with `LocalFsBackend` as the CLI default and the routing proven by SpyBackend contract tests.**

## What Happened

M012 had one slice (S01) executed across three tasks. T01 established the contract test framework: a `SpyBackend` test double and two red-state tests defining the routing behavior. T02 implemented the production code: `GuardDaemon` gained a `backend: Arc<dyn StateBackend>` field, `save_checkpoint_routed()` was extracted as a testable routing surface, and `start_guard()` got dual cfg-gated signatures. T03 wired the CLI and verified the full workspace.

The key design choices:

- **cfg-gated dual constructors**: `GuardDaemon::new()` has two signatures under `#[cfg(feature = "orchestrate")]` — one with backend (for orchestrated/remote use), one without (preserving backward compatibility). The `without` variant always uses `LocalFsBackend` internally.
- **`save_checkpoint_routed()` as testable surface**: Rather than making `try_save_checkpoint` testable by routing through extract_team_state (which requires a real session JSONL), a dedicated `save_checkpoint_routed()` method takes a `&TeamCheckpoint` directly — the contract tests construct the checkpoint explicitly and verify the routing without filesystem coupling.
- **CLI default**: `handle_guard_start` passes `Arc::new(LocalFsBackend::new(&assay_dir))` — zero behavior change for local users (D175).

## Cross-Slice Verification

| Success criterion | Evidence |
|---|---|
| `GuardDaemon` accepts `backend: Arc<dyn StateBackend>` at construction | `GuardDaemon::new()` under `#[cfg(feature = "orchestrate")]` takes backend; all tests pass |
| `try_save_checkpoint` routes through backend when `supports_checkpoints=true` | `test_checkpoint_routing_calls_backend_when_capability_true` passes; SpyBackend count == 1 |
| `try_save_checkpoint` falls back to local when `supports_checkpoints=false` | `test_checkpoint_routing_skips_backend_when_capability_false` passes; SpyBackend count == 0 |
| `start_guard()` signature updated; all call sites updated | Dual signatures in `guard/mod.rs`; CLI updated; all compilation passes |
| CLI `handle_guard_start` passes `LocalFsBackend` by default | Code in `context.rs` confirmed; `just ready` green |
| Contract tests prove routing with spy backend | Both SpyBackend routing tests pass |
| `just ready` green with 1526+ tests | 1529 tests pass, zero failures |
| D175 and D176 documented | Both rows present in DECISIONS.md |

## Requirement Changes

- R080: active → validated — `GuardDaemon.backend` field, `save_checkpoint_routed()` routing, `start_guard()` API, CLI default, SpyBackend contract tests, `just ready` green with 1529 tests.

## Forward Intelligence

### What the next milestone should know
- `save_checkpoint_routed()` is `pub(crate)` — it's the testable routing surface. The public entry point is `try_save_checkpoint()` which calls it after running extract_team_state.
- The orchestrate feature gate is used on both `GuardDaemon::new()` variants and `start_guard()`. The non-orchestrate build has the single-arg constructor that always uses LocalFsBackend internally.
- Orchestrators (executor.rs, mesh.rs, gossip.rs) still do NOT call `save_checkpoint_summary` — they lack `TeamCheckpoint` data. The guard daemon is the sole checkpoint extraction path by design.
- `SshSyncBackend::save_checkpoint_summary()` is the remote-worker use case this milestone enables; real multi-machine validation is UAT only.

### What's fragile
- The cfg-gated dual constructors add complexity. If the orchestrate feature is ever stabilized as always-on, collapse to a single constructor.
- `save_checkpoint_routed()` is `pub(crate)` for test access. If the guard module boundary changes, visibility may need adjustment.

### Authoritative diagnostics
- `cargo nextest run --package assay-core --features orchestrate "checkpoint_routing"` — the two contract tests
- `info!("[guard] Checkpoint saved via backend")` in production logs confirms backend routing is active

### What assumptions changed
- None. The planning assumptions (CLI defaults to LocalFsBackend, orchestrators don't call save_checkpoint_summary, sync-in-async risk accepted) all held.

## Files Created/Modified

- `crates/assay-core/src/guard/daemon.rs` — `GuardDaemon::new` backend parameter (cfg-gated); `save_checkpoint_routed()` routing method; `try_save_checkpoint` calls it; SpyBackend + routing contract unit tests
- `crates/assay-core/src/guard/mod.rs` — dual `start_guard()` signatures (cfg-gated)
- `crates/assay-cli/src/commands/context.rs` — `handle_guard_start` constructs and passes `LocalFsBackend` backend
- `.kata/milestones/M012/slices/S01/S01-SUMMARY.md` — slice summary (already committed)
- `.kata/milestones/M012/slices/S01/tasks/T01-SUMMARY.md` — task summary
- `.kata/milestones/M012/slices/S01/tasks/T02-SUMMARY.md` — task summary
- `.kata/milestones/M012/slices/S01/tasks/T03-SUMMARY.md` — task summary
- `.kata/milestones/M012/M012-SUMMARY.md` — this file
