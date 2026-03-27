# M012: Checkpoint Persistence on Remote Backends

**Vision:** Route `GuardDaemon` checkpoint writes through `Arc<dyn StateBackend>` so that smelt workers running on remote machines can push checkpoint state to whatever backend is configured. The `save_checkpoint_summary` method — which exists on the trait and is already implemented by `SshSyncBackend` — finally gets callers in production code. Two slices: plumb the backend into `GuardDaemon`, wire the CLI, and verify with contract tests.

## Success Criteria

- `GuardDaemon` accepts `backend: Arc<dyn StateBackend>` at construction; `try_save_checkpoint` routes through it when `capabilities().supports_checkpoints` is true; falls back to local `save_checkpoint` (today's behavior) when false
- `start_guard()` public API accepts an `Arc<dyn StateBackend>` parameter; all call sites updated
- CLI `handle_guard_start` passes `Arc::new(LocalFsBackend::new(assay_dir))` by default — behavior unchanged for local users
- Contract tests prove `save_checkpoint_summary` is called on the backend with the correct `TeamCheckpoint` argument via a spy/mock backend
- `just ready` green with all 1526+ tests passing — zero regression
- D175 documented: `GuardDaemon` backend plumbing decision

## Key Risks / Unknowns

- **`start_guard` API change is a breaking change** — it's a public function; all call sites (guard/mod.rs, the CLI, and tests) must be updated in one atomic slice. Missing any site causes a compile error.
- **Async context + sync trait call** — `GuardDaemon::run()` is async (tokio). Calling a sync `save_checkpoint_summary` from inside an async context is safe but must not block the executor for long; scp could take seconds on a slow link. Mitigation: document the known limitation; blocking in tests uses `NoopBackend` which is instant.

## Proof Strategy

- `start_guard` API change → retire in S01 by updating all call sites and verifying compile + test green
- sync-in-async blocking risk → retire in S01 by documenting it (D176) and noting that production SshSyncBackend calls are bounded by scp timeout; test uses NoopBackend

## Verification Classes

- Contract verification: spy/mock backend recording `save_checkpoint_summary` calls; contract test asserting the call happens when `supports_checkpoints = true` and does NOT happen when false; `just ready` green
- Integration verification: all existing guard daemon tests pass unchanged; `OrchestratorConfig::default()` tests pass unchanged
- Operational verification: guard daemon lifecycle tests (`try_save_checkpoint` triggers on guard-soft, guard-hard, guard-circuit-trip, guard-shutdown)
- UAT / human verification: run `assay context guard start` with a real session and confirm checkpoint file created; run with `SshSyncBackend` config and confirm remote file appears (manual only)

## Milestone Definition of Done

This milestone is complete only when all are true:

- `GuardDaemon.backend: Arc<dyn StateBackend>` field exists and is set at construction
- `try_save_checkpoint` calls `backend.save_checkpoint_summary()` when `supports_checkpoints = true`
- `try_save_checkpoint` calls `crate::checkpoint::save_checkpoint()` (local) when `supports_checkpoints = false` — preserving today's behavior for local users
- `start_guard()` signature includes `Arc<dyn StateBackend>` parameter; all call sites updated
- CLI `handle_guard_start` compiles and passes `Arc::new(LocalFsBackend::new(assay_dir))` — no behavior change for existing users
- Contract test asserting `save_checkpoint_summary` called → PASS; not called when capability false → PASS
- `just ready` green with 1526+ tests, zero failures
- D175 (GuardDaemon backend plumbing) appended to DECISIONS.md

## Requirement Coverage

- Covers: R080
- Partially covers: none
- Leaves for later: LinearBackend checkpoint support (R080 notes), real multi-machine UAT
- Orphan risks: none

## Slices

- [x] **S01: GuardDaemon backend plumbing and contract tests** `risk:high` `depends:[]`
  > After this: `GuardDaemon` accepts `Arc<dyn StateBackend>`; `try_save_checkpoint` routes through it; `start_guard()` API updated; CLI wired; contract tests proving routing with spy backend; `just ready` green with 1526+ tests.

## Boundary Map

### S01 → (M013)

Produces:
- `GuardDaemon::new(session_path, assay_dir, project_dir, config, backend: Arc<dyn StateBackend>) -> Self`
- `GuardDaemon.backend: Arc<dyn StateBackend>` field
- `try_save_checkpoint` routing: `if backend.capabilities().supports_checkpoints { backend.save_checkpoint_summary(...) } else { save_checkpoint(assay_dir, ...) }`
- `start_guard(session_path, assay_dir, project_dir, config, backend: Arc<dyn StateBackend>)` updated signature
- CLI `handle_guard_start` passing `Arc::new(LocalFsBackend::new(&assay))` — preserves existing behavior
- Contract test: spy backend records `save_checkpoint_summary` calls; asserted in test
- D175 and D176 decisions documented

Consumes:
- nothing new — builds on M011's `SshSyncBackend::save_checkpoint_summary()` and existing `GuardDaemon` infrastructure
