# S01: GuardDaemon backend plumbing and contract tests — UAT

**Milestone:** M012
**Written:** 2026-03-27

## UAT Type

- UAT mode: mixed (artifact-driven + human-experience)
- Why this mode is sufficient: Contract and integration tests cover all routing logic mechanically (SpyBackend + NoopBackend prove the conditional dispatch). The only gap requiring human validation is live runtime behavior with a real session and a real remote backend — this is a one-time integration smoke test, not a regression surface.

## Preconditions

- Assay built from this branch/commit
- An active Claude Code session with at least one tool call (produces a `.claude/projects/` session file)
- For the remote test only: an SSH-accessible remote host with scp working

## Smoke Test

Run `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_` and confirm both contract tests pass. This confirms the routing logic is wired correctly for the standard case.

## Test Cases

### 1. Local checkpoint created on guard start (no behavior change)

1. Navigate to an assay project with an active Claude Code session
2. Run: `assay context guard start`
3. Trigger a guard soft event (or wait for a session threshold)
4. **Expected:** A checkpoint file appears under `.assay/checkpoints/`; no errors in stderr; log shows `[guard] Checkpoint saved: <path>`

### 2. Backend routing when supports_checkpoints = true (contract test)

1. Run: `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_backend_called_when_supports_checkpoints`
2. **Expected:** Test passes; spy backend has recorded exactly 1 `TeamCheckpoint` call

### 3. No backend call when supports_checkpoints = false (contract test)

1. Run: `cargo test -p assay-core --features orchestrate -- guard::daemon::tests::contract_backend_not_called_when_no_checkpoint_capability`
2. **Expected:** Test passes; local checkpoint path was taken; no spy call

### 4. Non-orchestrate build unchanged

1. Run: `cargo test -p assay-core -- guard::daemon::tests`
2. **Expected:** 9 tests pass; no backend field visible; same behavior as pre-M012

## Edge Cases

### SshSyncBackend remote checkpoint (manual — requires remote machine)

1. Configure a RunManifest with `state_backend = { type = "ssh", host = "<remote>", remote_assay_dir = "/path/to/.assay" }`
2. Start a guard with SshSyncBackend passed to `start_guard()`
3. Trigger a checkpoint event
4. **Expected:** Remote file appears at `<remote>:/path/to/.assay/checkpoints/<session-id>.json`; local log shows `[guard] Checkpoint saved via backend`

### Backend save failure is non-fatal

1. Configure a backend that returns `Err` from `save_checkpoint_summary`
2. Trigger a checkpoint event
3. **Expected:** Guard daemon continues running; log shows `[guard] Backend checkpoint save failed: <error>`; no panic; no daemon exit

## Failure Signals

- `[guard] Backend checkpoint save failed` with no subsequent `[guard] Checkpoint saved via backend` — backend error path taken (non-fatal)
- Contract tests fail to compile — `GuardDaemon::new()` signature mismatch; check T02 changes were applied
- `assay-cli` fails to build — `handle_guard_start` signature mismatch; check context.rs imports
- `just ready` reports failures — regression in daemon tests

## Requirements Proved By This UAT

- R080 — `GuardDaemon` accepts `Arc<dyn StateBackend>`; `try_save_checkpoint` routes through backend when `supports_checkpoints = true`; falls back to local when false; `start_guard()` API extended; CLI passes `LocalFsBackend` by default; contract tests (cases 2 and 3 above) prove the routing. Automated tests + `just ready` green = R080 validated.

## Not Proven By This UAT

- Real SshSyncBackend checkpoint push to a remote machine — requires a live remote host (manual only, case listed under Edge Cases)
- `save_checkpoint_summary` blocking behavior under a slow scp link — D176 accepted risk; would require artificial delay injection
- LinearBackend and GitHubBackend checkpoints — both have `supports_checkpoints = false` by design (R080 notes); no checkpoint routing expected through them

## Notes for Tester

- The automated contract tests (cases 2 and 3) are the primary proof surface. The live runtime test (case 1) is a sanity check for the CLI wiring.
- D176 documents the accepted risk of blocking the tokio thread during `save_checkpoint_summary`. For local testing with `LocalFsBackend`, the write is instant. Only observable with a slow remote backend.
- The non-unix stub `handle_guard_start` (compiled on non-unix targets) correctly returns an error without calling `start_guard`, requiring no changes.
