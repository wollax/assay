---
id: T02
parent: S03
milestone: M008
provides:
  - sync_state_back() free function in ssh.rs for remote-to-local state directory sync
  - Mock-based unit test for success path (test_sync_state_back_mock_success)
  - Mock-based unit test for failure path (test_sync_state_back_mock_failure)
  - Gated integration test for full state sync round-trip (test_state_sync_round_trip)
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - "sync_state_back computes remote path as /tmp/.smelt/runs/<job_name>/ — uses job_name (from manifest), not JobId"
patterns_established:
  - "sync_state_back follows the same pattern as deliver_manifest/run_remote_job: free function generic over SshClient trait, with tracing debug/warn"
  - "Local directory structure mirrors remote: local_dest_dir/.smelt/runs/<job_name>/"
observability_surfaces:
  - "tracing::debug on sync_state_back entry (host, job_name, local_dest_dir)"
  - "tracing::warn on sync_state_back failure (job_name, host, error)"
  - "anyhow::Error propagated to caller with full scp error context"
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T02: Implement sync_state_back() free function with unit tests and gated integration test

**Added sync_state_back() free function that creates local state dirs and calls scp_from() to pull remote .smelt/runs/<job_name>/ back, with 2 mock unit tests and 1 gated integration test**

## What Happened

Implemented `sync_state_back()` as a public async free function in `ssh.rs`, generic over `SshClient`. The function computes the remote source path from `job_name` (`/tmp/.smelt/runs/<job_name>/`), creates the local target directory tree (`local_dest_dir/.smelt/runs/<job_name>/`) via `create_dir_all`, then delegates to `scp_from()`. Added tracing debug on entry and warn on failure with job_name and host for correlation.

Added two mock-based unit tests in `ssh.rs::tests`:
- `test_sync_state_back_mock_success` — verifies Ok return and local directory creation
- `test_sync_state_back_mock_failure` — verifies Err propagation from scp_from

Added gated integration test `test_state_sync_round_trip` in `tests.rs` — creates remote state via ssh exec, calls sync_state_back to pull it locally, verifies the file exists and is valid TOML with correct job_name and phase fields. Cleans up remote dir on completion.

## Verification

- `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back` — 2 passed
- `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_from_args` — 1 passed (slice verification)
- `cargo test --workspace` — 70 passed, 4 ignored, 0 failed in smelt-cli lib tests. 1 pre-existing failure in docker_lifecycle (test_cli_run_invalid_manifest) unrelated to this change.
- `test_state_sync_round_trip` is `#[ignore]`d and present in tests.rs

## Diagnostics

- Search logs for `sync_state_back entry` to see invocations (host, job_name, local_dest_dir)
- Search logs for `sync_state_back failed` to see failures (job_name, host, error)
- Error messages propagate the full scp_from error including host, exit code, and stderr

## Deviations

None.

## Known Issues

- Pre-existing test failure: `test_cli_run_invalid_manifest` in docker_lifecycle.rs (unrelated to S03 work)

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — Added sync_state_back() free function + 2 mock unit tests
- `crates/smelt-cli/src/serve/tests.rs` — Added test_state_sync_round_trip gated integration test
