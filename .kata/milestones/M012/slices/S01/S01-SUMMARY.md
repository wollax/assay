---
id: S01
parent: M012
milestone: M012
provides:
  - GuardDaemon backend field (Arc<dyn StateBackend>) behind orchestrate feature gate
  - Dual GuardDaemon::new() constructors (with/without backend) via cfg gates
  - save_checkpoint_routed() — testable routing surface for backend vs local checkpoint path
  - Dual start_guard() signatures in mod.rs with orchestrate gate
  - SpyBackend test double recording save_checkpoint_summary calls
  - Two contract tests proving routing when supports_checkpoints true/false
  - CLI handle_guard_start wired with Arc<LocalFsBackend> — no behavior change for users
  - R080 validated: full checkpoint routing chain operational
requires: []
affects: []
key_files:
  - crates/assay-core/src/guard/daemon.rs
  - crates/assay-core/src/guard/mod.rs
  - crates/assay-cli/src/commands/context.rs
key_decisions:
  - D175 — GuardDaemon accepts Arc<dyn StateBackend>; CLI defaults to LocalFsBackend
  - D176 — save_checkpoint_summary called synchronously inside async GuardDaemon run loop
patterns_established:
  - cfg-gated dual constructors and dual public API functions at orchestrate feature boundary
  - save_checkpoint_routed() as a testable routing surface separate from extraction (avoids real-session filesystem dependency in contract tests)
  - SpyBackend records calls via Arc<Mutex<Vec<TeamCheckpoint>>> for assertion
  - Contract tests construct TeamCheckpoint directly and call routing method; no filesystem dependency on ~/.claude/ session discovery
observability_surfaces:
  - "info!('[guard] Checkpoint saved via backend') when backend path taken"
  - "warn!('[guard] Backend checkpoint save failed: {e}') on backend error (non-fatal)"
  - "info!('[guard] Checkpoint saved: {}') for local fallback path"
drill_down_paths:
  - .kata/milestones/M012/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M012/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M012/slices/S01/tasks/T03-SUMMARY.md
duration: 3 tasks (~1h15m)
verification_result: passed
completed_at: 2026-03-27
---

# S01: GuardDaemon backend plumbing and contract tests

**`GuardDaemon` accepts `Arc<dyn StateBackend>` behind orchestrate feature gate; `try_save_checkpoint` routes through backend when `supports_checkpoints = true`; `start_guard()` API updated; CLI wired with `LocalFsBackend`; contract tests prove routing via SpyBackend; `just ready` green with 1503 tests, zero failures.**

## What Happened

Three tasks executed in order:

**T01 (red state):** Added `SpyBackend` test double to the daemon test module. SpyBackend implements all 7 `StateBackend` methods; `save_checkpoint_summary` records calls into `Arc<Mutex<Vec<TeamCheckpoint>>>`. Added two contract tests — `contract_backend_called_when_supports_checkpoints` and `contract_backend_not_called_when_no_checkpoint_capability` — that intentionally failed to compile because `GuardDaemon::new()` didn't yet accept a backend. This established the expected API contract as failing code before implementation.

**T02 (green state):** Added `#[cfg(feature = "orchestrate")] backend: Arc<dyn StateBackend>` field to `GuardDaemon`. Provided two cfg-gated `new()` constructors — the orchestrate variant accepts `Arc<dyn StateBackend>` as a fifth argument, the non-orchestrate variant retains the original 4-param signature. Extracted `save_checkpoint_routed()` as a separate method from `try_save_checkpoint()` — this allows contract tests to test routing with synthetic `TeamCheckpoint` values without requiring real session discovery from `~/.claude/projects/`. The routing logic: when orchestrate enabled and `backend.capabilities().supports_checkpoints` is true, calls `backend.save_checkpoint_summary()`; otherwise falls through to the existing local `save_checkpoint()`. Updated `start_guard()` in `mod.rs` with dual cfg-gated signatures. Updated `make_daemon()` test helper to pass `Arc::new(NoopBackend)` under orchestrate. CLI wiring (`handle_guard_start` → `Arc::new(LocalFsBackend::new(assay.clone()))`) was completed in T02 alongside the API change since assay-cli always enables orchestrate.

**T03 (verification):** Confirmed all slice-level checks pass. CLI wiring already done in T02. Ran full workspace: `cargo test -p assay-cli` → 52 passed; `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` → 11 passed (9 existing + 2 contract tests); non-orchestrate → 9 passed; `just ready` → all checks green.

## Verification

- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests` — **11 passed** (9 existing + 2 contract tests)
- `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_` — both contract tests pass
- `cargo test -p assay-core -- guard::daemon::tests` — **9 passed** (non-orchestrate build compiles and passes)
- `cargo test -p assay-cli` — **52 passed**
- `cargo test --workspace` — **1503 passed**, 0 failed
- `just ready` — **all checks passed** (fmt, lint, test, deny, plugin version)

## Requirements Advanced

- R080 — `GuardDaemon` now accepts `Arc<dyn StateBackend>`; `try_save_checkpoint` routes through backend; `start_guard()` API updated; CLI wired; contract tests prove routing — moved from Active to Validated

## Requirements Validated

- R080 — Contract tests prove (1) spy backend's `save_checkpoint_summary` is called with correct `TeamCheckpoint` when `supports_checkpoints = true`, (2) spy backend is NOT called when `supports_checkpoints = false` and local `save_checkpoint` runs instead. All 1503 workspace tests pass. Full call chain operational: CLI → `start_guard()` → `GuardDaemon::new()` → `try_save_checkpoint()` → `save_checkpoint_routed()` → backend.

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- **`save_checkpoint_routed()` extracted as separate method** — the plan had routing logic inline in `try_save_checkpoint()`, but contract tests needed to bypass `extract_team_state()` (which requires real Claude session discovery via `~/.claude/projects/`). Routing logic is identical; only the entry point for testing differs. Both methods are tested — `try_save_checkpoint` is tested indirectly via existing lifecycle tests; `save_checkpoint_routed` is tested directly by contract tests.
- **CLI wiring completed in T02, not T03** — because assay-cli always enables the orchestrate feature, the signature mismatch was caught at `cargo check` time when updating `start_guard()`. Wiring in T02 was the natural path; T03 focused on verification.

## Known Limitations

- `save_checkpoint_summary` is called synchronously inside `GuardDaemon::run()` (async tokio context). On a slow SshSyncBackend, scp could block the tokio thread for up to ~30s per checkpoint event. Documented in D176; accepted risk given checkpoint infrequency. Mitigation: wrap in `spawn_blocking` if latency becomes observable.
- Real multi-machine UAT (SshSyncBackend with a remote host) is not covered by automated tests — it's manual-only and requires an accessible remote machine.

## Follow-ups

- M013: LinearBackend and GitHubBackend keep `supports_checkpoints = false` (per R080 notes). No checkpoint routing changes needed in those backends.
- Future CLI extension: `--backend` flag for `assay context guard start` if users want non-default backends without code changes.
- If guard latency becomes observable: wrap `save_checkpoint_summary` call in `spawn_blocking` (D176 revisit trigger).

## Files Created/Modified

- `crates/assay-core/src/guard/daemon.rs` — Added `backend` field, dual constructors, `save_checkpoint_routed()`, updated `try_save_checkpoint()`, updated `make_daemon()` and `guard_daemon_new_creates_valid_struct` test helpers, SpyBackend struct, two contract tests
- `crates/assay-core/src/guard/mod.rs` — Dual `start_guard()` signatures with orchestrate feature gate, Arc/StateBackend imports
- `crates/assay-cli/src/commands/context.rs` — `handle_guard_start` passes `Arc::new(LocalFsBackend::new(assay.clone()))` to `start_guard()`

## Forward Intelligence

### What the next slice should know
- M012 has only one slice (S01). This slice completes the milestone. No remaining slices.
- The `save_checkpoint_routed()` method is the testable surface for checkpoint routing — future tests that need to verify routing behavior should call this method directly with a synthetic `TeamCheckpoint` rather than calling `try_save_checkpoint()` which requires real session discovery.
- `OrchestratorConfig::default()` uses a placeholder `.assay` path for `LocalFsBackend` — this is intentional (D157) to avoid cascading changes to ~20 test sites.

### What's fragile
- The non-orchestrate feature path has fewer tests (9 vs 11). Any new daemon functionality added under orchestrate must check the non-orchestrate build still compiles.
- `make_daemon()` has a cfg-gated variant — if the function signature changes, both variants must be updated.

### Authoritative diagnostics
- Grep guard logs for `Checkpoint saved via backend` to confirm backend routing is active at runtime
- Grep for `Backend checkpoint save failed` to detect non-fatal backend errors
- Grep for `Checkpoint saved:` (with path) to confirm local fallback path is used

### What assumptions changed
- Original plan assumed CLI wiring would be a distinct T03 task; in practice the compiler enforced it during T02's `start_guard()` signature change, making T03 purely verification work.
