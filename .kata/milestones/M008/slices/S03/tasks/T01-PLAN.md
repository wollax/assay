---
estimated_steps: 5
estimated_files: 1
---

# T01: Add scp_from() to SshClient trait with recursive copy and MockSshClient extension

**Slice:** S03 — State sync back via scp
**Milestone:** M008

## Description

Extend the `SshClient` trait with a `scp_from()` method that copies a remote directory to the local filesystem recursively. Implement it in `SubprocessSshClient` using the existing `build_scp_args()` helper with `-r` for recursive copy and reversed argument order (remote source first, local destination second). Extend `MockSshClient` with a dedicated `scp_from_results` queue so tests can configure scp_from outcomes independently from scp_to results.

## Steps

1. Add `scp_from(&self, worker: &WorkerConfig, timeout_secs: u64, remote_src: &str, local_dest: &std::path::Path) -> anyhow::Result<()>` to the `SshClient` trait in `ssh.rs`
2. Implement `SubprocessSshClient::scp_from()` — resolve `scp` binary via `scp_binary()`, build args via `build_scp_args(worker, timeout_secs, &["-r", &remote_spec, &local_str])` where `remote_spec = format!("{}@{}:{}", worker.user, worker.host, remote_src)`; check exit code, warn on failure, return `Err` on non-zero exit
3. Add `scp_from_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>` to `MockSshClient`; initialize in `new()`; add `with_scp_from_result(self, result) -> Self` builder; implement the `scp_from` trait method to pop from this queue
4. Add unit test `test_scp_from_args_recursive` — call `build_scp_args` with `-r` in extra_args and verify the output contains `-r`, the remote spec comes before the local path, and `-P` is used (not `-p`) for custom port
5. Add unit test `test_scp_from_mock_success` — configure mock with `Ok(())`, call scp_from, assert success; and `test_scp_from_mock_failure` — configure mock with `Err`, call scp_from, assert error returned

## Must-Haves

- [ ] `scp_from()` method exists on `SshClient` trait with correct signature
- [ ] `SubprocessSshClient::scp_from()` uses `-r` flag for recursive copy
- [ ] `SubprocessSshClient::scp_from()` uses remote-first arg order: `user@host:/remote/path /local/path`
- [ ] `MockSshClient` has separate `scp_from_results` queue (not shared with `scp_results`)
- [ ] `test_scp_from_args_recursive` passes — verifies `-r` flag present
- [ ] `test_scp_from_mock_success` and `test_scp_from_mock_failure` pass
- [ ] `cargo test --workspace` all green

## Verification

- `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_from` — runs all scp_from-related tests
- `cargo test --workspace` — no regressions

## Observability Impact

- Signals added/changed: `tracing::debug!` on scp_from entry (host, remote_src, local_dest); `tracing::warn!` on non-zero exit (host, exit_code, stderr) — mirrors scp_to pattern
- How a future agent inspects this: grep logs for `scp_from` entries; stderr captured in error message
- Failure state exposed: anyhow::Error with host, exit_code, and stderr content

## Inputs

- `crates/smelt-cli/src/serve/ssh.rs` — existing SshClient trait, SubprocessSshClient, build_scp_args(), MockSshClient with scp_results queue
- S02 summary: `scp_to()` implementation pattern is the model; build_scp_args uses uppercase `-P` for port

## Expected Output

- `crates/smelt-cli/src/serve/ssh.rs` — extended with scp_from trait method, SubprocessSshClient::scp_from(), MockSshClient::scp_from_results queue and builder, 3 new unit tests
