---
estimated_steps: 5
estimated_files: 3
---

# T02: Add deliver_manifest + run_remote_job + integration test + remove dead_code

**Slice:** S02 — Manifest delivery + remote smelt run execution
**Milestone:** M008

## Description

Implement the two boundary-producing free functions: `deliver_manifest<C: SshClient>()` (scp manifest to `/tmp/smelt-<job_id>.toml`) and `run_remote_job<C: SshClient>()` (SSH exec `smelt run <path>`, return raw exit code). Add 3 mock-based unit tests proving contract, 1 gated integration test proving real localhost SSH path. Remove `#[allow(dead_code)]` from fields in `config.rs` that are now consumed.

## Steps

1. Add `deliver_manifest<C: SshClient>()` free function at the bottom of ssh.rs:
   - Signature: `pub async fn deliver_manifest<C: SshClient>(client: &C, worker: &WorkerConfig, timeout_secs: u64, job_id: &JobId, local_manifest: &std::path::Path) -> anyhow::Result<String>`
   - Compute `remote_path = format!("/tmp/smelt-{}.toml", job_id)`
   - Compute `remote_dest = format!("{}@{}:{}", worker.user, worker.host, remote_path)`
   - Call `client.scp_to(worker, timeout_secs, local_manifest, &remote_dest).await?`
   - Return `Ok(remote_path)`
   - Import `JobId` from `crate::serve::types`

2. Add `run_remote_job<C: SshClient>()` free function:
   - Signature: `pub async fn run_remote_job<C: SshClient>(client: &C, worker: &WorkerConfig, timeout_secs: u64, remote_manifest_path: &str) -> anyhow::Result<i32>`
   - Build command: `format!("smelt run {}", remote_manifest_path)`
   - Call `client.exec(worker, timeout_secs, &cmd).await?`
   - If `exit_code == 127`, emit `tracing::warn!` with host and hint: "smelt may not be on the remote PATH"
   - Return `Ok(output.exit_code)` — raw, no mapping (D050)

3. Add 3 mock-based unit tests in ssh.rs `#[cfg(test)]` block:
   - `test_deliver_manifest_mock`: push Ok(()) to scp_results; call deliver_manifest; assert returns correct remote path string `/tmp/smelt-job-1.toml`
   - `test_run_remote_job_mock_success`: push Ok(SshOutput { exit_code: 0, .. }) to exec_results; call run_remote_job; assert returns 0
   - `test_run_remote_job_mock_exit2`: push Ok(SshOutput { exit_code: 2, .. }) to exec_results; call run_remote_job; assert returns 2

4. Add gated integration test `test_manifest_delivery_and_remote_exec` in `crates/smelt-cli/src/serve/tests.rs`:
   - Guard: `#[tokio::test] #[ignore]` + `SMELT_SSH_TEST=1` env check
   - Create a temp file with a minimal valid manifest TOML (use `VALID_MANIFEST_TOML`)
   - Use SubprocessSshClient + localhost WorkerConfig (same pattern as test_ssh_exec_localhost)
   - Call `deliver_manifest()` → assert Ok, get remote_path
   - Verify file exists on remote: `client.exec(worker, 5, &format!("test -f {remote_path}"))` → exit_code 0
   - Call `run_remote_job()` with `--dry-run` flag appended: `smelt run --dry-run <remote_path>` — this avoids needing Docker on the test host
   - Assert exit_code == 0 AND stderr does not contain "not found" (clear diagnostic per research)
   - Clean up: `client.exec(worker, 5, &format!("rm -f {remote_path}"))` — best-effort

5. Remove `#[allow(dead_code)]` from `config.rs`:
   - Remove annotation from `key_env` field on WorkerConfig
   - Remove annotation from `port` field on WorkerConfig
   - Remove annotation from `ssh_timeout_secs` field on ServerConfig
   - Leave the `retry_backoff_secs` dead_code annotation (still unused)

## Must-Haves

- [ ] `deliver_manifest<C>()` returns `/tmp/smelt-<job_id>.toml` string
- [ ] `run_remote_job<C>()` returns raw i32 exit code, no mapping
- [ ] Exit code 127 emits WARN log about remote PATH
- [ ] `test_deliver_manifest_mock` passes
- [ ] `test_run_remote_job_mock_success` passes
- [ ] `test_run_remote_job_mock_exit2` passes
- [ ] `test_manifest_delivery_and_remote_exec` gated integration test added
- [ ] `#[allow(dead_code)]` removed from key_env, port, ssh_timeout_secs
- [ ] `cargo test --workspace` all green

## Verification

- `cargo test -p smelt-cli -- test_deliver_manifest_mock` — passes
- `cargo test -p smelt-cli -- test_run_remote_job_mock` — 2 tests pass
- `cargo test --workspace` — all pass, 0 failures
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` — returns 1 (only retry_backoff_secs)

## Observability Impact

- Signals added/changed: `tracing::warn!` on exit code 127 with "smelt may not be on the remote PATH" hint — surfaces the most common remote exec failure clearly
- How a future agent inspects this: check serve.log for "not on the remote PATH" warnings when SSH exec returns 127; check SshOutput.stderr for "command not found"
- Failure state exposed: deliver_manifest failure → anyhow Err with scp details; run_remote_job exit 127 → WARN log; all exit codes propagated transparently

## Inputs

- `crates/smelt-cli/src/serve/ssh.rs` — T01's scp_to(), build_scp_args(), MockSshClient
- `crates/smelt-cli/src/serve/types.rs` — JobId with Display trait (formats as `job-N`)
- `crates/smelt-cli/src/serve/config.rs` — WorkerConfig, ServerConfig (dead_code to remove)
- `crates/smelt-cli/src/serve/tests.rs` — VALID_MANIFEST_TOML constant, existing gated test patterns

## Expected Output

- `crates/smelt-cli/src/serve/ssh.rs` — deliver_manifest(), run_remote_job(), 3 new mock unit tests
- `crates/smelt-cli/src/serve/config.rs` — 3 dead_code annotations removed
- `crates/smelt-cli/src/serve/tests.rs` — 1 new gated integration test
