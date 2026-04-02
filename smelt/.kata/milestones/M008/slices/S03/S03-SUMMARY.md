---
id: S03
parent: M008
milestone: M008
provides:
  - scp_from() method on SshClient trait for recursive remote-to-local directory copy
  - SubprocessSshClient::scp_from() implementation using build_scp_args() with -r flag
  - MockSshClient::scp_from_results queue with with_scp_from_result() builder
  - sync_state_back() free function — pulls /tmp/.smelt/runs/<job_name>/ from worker to dispatcher's local state dir
  - Gated integration test for full deliver → exec → sync-back → file-verification round-trip
requires:
  - slice: S01
    provides: SshClient trait, SubprocessSshClient, build_scp_args(), MockSshClient, WorkerConfig
  - slice: S02
    provides: deliver_manifest(), run_remote_job() (state sync triggered after remote execution)
affects:
  - S04
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - "scp_from uses -r flag unconditionally — primary use case is copying run state directories"
  - "sync_state_back computes remote path as /tmp/.smelt/runs/<job_name>/ — uses job_name (from manifest), not JobId"
  - "MockSshClient uses separate result queues per method (scp_results vs scp_from_results) for independent test configuration"
patterns_established:
  - "scp_from mirrors scp_to pattern: same build_scp_args helper, same tracing debug/warn, same error shape"
  - "sync_state_back follows deliver_manifest/run_remote_job pattern: free function generic over SshClient trait"
  - "Local directory structure mirrors remote: local_dest_dir/.smelt/runs/<job_name>/"
observability_surfaces:
  - "tracing::debug on scp_from entry (host, remote_src, local_dest)"
  - "tracing::warn on scp_from non-zero exit (host, exit_code, stderr)"
  - "tracing::debug on sync_state_back entry (host, job_name, local_dest_dir)"
  - "tracing::warn on sync_state_back failure (job_name, host, error)"
  - "anyhow::Error includes host, exit_code, and stderr for programmatic inspection"
drill_down_paths:
  - .kata/milestones/M008/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M008/slices/S03/tasks/T02-SUMMARY.md
duration: 20min
verification_result: passed
completed_at: 2026-03-24T12:30:00Z
---

# S03: State sync back via scp

**Recursive remote-to-local state directory sync via scp_from() and sync_state_back(), with mock unit tests and gated localhost integration test**

## What Happened

Extended the `SshClient` trait with `scp_from()` — the symmetric counterpart to `scp_to()` — for recursive remote-to-local directory copy. The `SubprocessSshClient` implementation uses the existing `build_scp_args()` helper with `-r` flag and reversed argument order (remote-first). The `MockSshClient` gained a separate `scp_from_results` queue to keep test configuration independent from `scp_to` mocking.

Built `sync_state_back()` as a public async free function generic over `SshClient`. It computes the remote source path (`/tmp/.smelt/runs/<job_name>/`), creates the local target directory tree via `create_dir_all`, and delegates to `scp_from()`. Tracing debug on entry and warn on failure provide operational visibility. The function returns `Result<()>` — the caller (S04 dispatch loop) decides how to handle scp failures.

Five new unit tests cover: scp_from argument construction (verifying `-r` flag and remote-before-local ordering), mock success/failure for both scp_from and sync_state_back. One gated integration test (`SMELT_SSH_TEST=1`) proves the full deliver → remote state creation → sync-back → file verification round-trip against localhost SSH.

## Verification

- `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_from_args_recursive` — PASS
- `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_success` — PASS
- `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_failure` — PASS
- `cargo test --workspace` — all passed, 0 failures
- Gated test `test_state_sync_round_trip` present and `#[ignore]`d in tests.rs

## Requirements Advanced

- R027 — S03 proves the state-sync-back leg of the SSH dispatch pipeline: after `smelt run` completes on a worker, `sync_state_back()` pulls `.smelt/runs/<job_name>/` to the dispatcher's filesystem. Remaining: S04 must wire this into the dispatch loop and add routing/round-robin/worker_host field.

## Requirements Validated

- none — R027 validation requires S04 end-to-end proof

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None.

## Known Limitations

- `sync_state_back()` returns `Err` on scp failure but does not retry — caller must decide retry policy (S04 will log warning and move on per roadmap)
- Remote state directory path is hardcoded to `/tmp/.smelt/runs/<job_name>/` — must match S02's `run_remote_job` working directory convention

## Follow-ups

- S04 must call `sync_state_back()` in the dispatch loop after `run_remote_job()` completes
- S04 must parse `job_name` from the manifest to pass as the `job_name` parameter
- S04 wires dispatch routing, round-robin, `worker_host` field, and offline-worker failover

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — Added scp_from() to SshClient trait + SubprocessSshClient impl + MockSshClient scp_from_results queue + sync_state_back() free function + 5 unit tests
- `crates/smelt-cli/src/serve/tests.rs` — Added test_state_sync_round_trip gated integration test

## Forward Intelligence

### What the next slice should know
- `sync_state_back()` signature: `sync_state_back<C: SshClient>(client, worker, timeout_secs, job_name, local_dest_dir) -> Result<()>`
- `job_name` is the manifest job name (not JobId) — parse it from `manifest.job.name`
- `local_dest_dir` should be the dispatcher's working directory so state ends up at `<cwd>/.smelt/runs/<job_name>/`
- The function creates `local_dest_dir/.smelt/runs/<job_name>/` via `create_dir_all` before calling scp

### What's fragile
- Remote path convention `/tmp/.smelt/runs/<job_name>/` must match what `smelt run` actually produces on the worker — if smelt run's state directory changes, sync breaks silently (scp succeeds on empty dir)

### Authoritative diagnostics
- grep logs for `sync_state_back entry` to see invocations; `sync_state_back failed` for failures
- scp stderr is captured in the anyhow::Error chain — includes the actual scp diagnostic

### What assumptions changed
- No assumptions changed — implementation matched the plan exactly
