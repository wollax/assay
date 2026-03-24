---
estimated_steps: 5
estimated_files: 2
---

# T02: Implement sync_state_back() free function with unit tests and gated integration test

**Slice:** S03 — State sync back via scp
**Milestone:** M008

## Description

Create the `sync_state_back()` free function that S04 dispatch will call after `run_remote_job()` completes. It computes the remote state path from the `job_name` parameter (not job_id), creates local directories, and calls `scp_from()` to pull the remote `.smelt/runs/<job_name>/` directory. Add mock-based unit tests for success and failure, and a gated integration test that proves the full deliver → exec → sync-back → state-read round-trip on localhost SSH.

## Steps

1. Add `pub async fn sync_state_back<C: SshClient>(client: &C, worker: &WorkerConfig, timeout_secs: u64, job_name: &str, local_dest_dir: &std::path::Path) -> anyhow::Result<()>` to `ssh.rs` — compute `remote_src = format!("/tmp/.smelt/runs/{}/", job_name)`, compute `local_target = local_dest_dir.join(".smelt/runs").join(job_name)`, call `std::fs::create_dir_all(&local_target)`, then call `client.scp_from(worker, timeout_secs, &remote_src, &local_target)`. Add `tracing::debug!` on entry, `tracing::warn!` on failure.
2. Add unit test `test_sync_state_back_mock_success` — create a `MockSshClient` with `with_scp_from_result(Ok(()))`, call `sync_state_back` with a tempdir as `local_dest_dir` and `job_name = "test-job"`, assert `Ok(())` returned and `local_dest_dir/.smelt/runs/test-job/` directory was created
3. Add unit test `test_sync_state_back_mock_failure` — configure mock with `Err(anyhow!("scp failed"))`, call `sync_state_back`, assert `Err` returned
4. Add gated integration test `test_state_sync_round_trip` in `tests.rs` — requires `SMELT_SSH_TEST=1`; create a tempdir, use `SubprocessSshClient` to `exec` a command on localhost that creates `/tmp/.smelt/runs/sync-test-<random>/state.toml` with valid TOML content (job_name, phase = "complete", etc.); call `sync_state_back` to pull it to the tempdir; verify the local file exists and is valid TOML; clean up remote dir via ssh exec `rm -rf`
5. Run `cargo test --workspace` to verify no regressions

## Must-Haves

- [ ] `sync_state_back()` exported as `pub` from `ssh.rs`
- [ ] Remote path derived from `job_name` parameter: `/tmp/.smelt/runs/<job_name>/`
- [ ] Local destination created via `create_dir_all` before scp
- [ ] `scp_from()` called with correct remote path and local target
- [ ] Unit test `test_sync_state_back_mock_success` passes and verifies local dir creation
- [ ] Unit test `test_sync_state_back_mock_failure` passes and verifies Err propagation
- [ ] Gated integration test `test_state_sync_round_trip` present and `#[ignore]`d
- [ ] `cargo test --workspace` all green

## Verification

- `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back` — runs both mock tests
- `cargo test --workspace` — all green
- Gated: `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_state_sync_round_trip`

## Observability Impact

- Signals added/changed: `tracing::debug!` on sync_state_back entry (worker host, job_name, local_dest_dir); `tracing::warn!` on failure (job_name, worker host, error)
- How a future agent inspects this: search logs for `sync_state_back`; error message includes job_name and worker host for correlation
- Failure state exposed: `anyhow::Error` propagated to caller with full scp error context; caller (S04) decides whether to mark job Failed or log warning

## Inputs

- `crates/smelt-cli/src/serve/ssh.rs` — `SshClient::scp_from()` from T01; `MockSshClient::with_scp_from_result()` from T01; `WorkerConfig` from S01; `deliver_manifest()` and `run_remote_job()` from S02 (used in integration test context)
- `crates/smelt-core/src/monitor.rs` — `JobMonitor::read()` for integration test state verification
- Research finding: remote state path is `/tmp/.smelt/runs/<job_name>/state.toml` where `job_name` is from the manifest's `[job] name` field, not the queue's `JobId`

## Expected Output

- `crates/smelt-cli/src/serve/ssh.rs` — `sync_state_back()` free function + 2 mock-based unit tests
- `crates/smelt-cli/src/serve/tests.rs` — `test_state_sync_round_trip` gated integration test
