---
id: T02
parent: S02
milestone: M008
provides:
  - deliver_manifest<C: SshClient>() free function returning /tmp/smelt-<job_id>.toml path
  - run_remote_job<C: SshClient>() free function returning raw i32 exit code
  - 3 mock-based unit tests (deliver_manifest_mock, run_remote_job_mock_success, run_remote_job_mock_exit2)
  - gated integration test test_manifest_delivery_and_remote_exec
  - dead_code annotations removed from key_env and port on WorkerConfig
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - Kept #[allow(dead_code)] on ssh_timeout_secs since it's on ServerConfig and not consumed until S04 dispatch routing; plan expected removal but compiler warns
  - Integration test uses client.exec() for dry-run instead of run_remote_job() since run_remote_job hardcodes `smelt run <path>` without --dry-run flag
patterns_established:
  - MockSshClient builder pattern: new() → with_exec_result() → with_scp_result() for configurable pop-front results
  - build_scp_args mirrors build_ssh_args but uses uppercase -P for port
  - Free functions generic over C: SshClient compose trait methods into higher-level operations
observability_surfaces:
  - tracing::warn! on exit code 127 with "smelt may not be on the remote PATH" hint
  - tracing::warn! + tracing::debug! on scp_to entry/failure with host, exit_code, stderr
duration: 15m
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T02: Add deliver_manifest + run_remote_job + integration test + remove dead_code

**Added deliver_manifest() and run_remote_job() free functions with mock unit tests, gated integration test, and dead_code cleanup on WorkerConfig fields**

## What Happened

Implemented both T01 and T02 deliverables since T01 had not been committed yet. Added `scp_to()` to the `SshClient` trait, `build_scp_args()` public helper (uppercase `-P` for port), `SubprocessSshClient::scp_to()` impl, and `MockSshClient` test double (T01 scope). Then added `deliver_manifest<C>()` which scps a manifest to `/tmp/smelt-<job_id>.toml` and returns the remote path, and `run_remote_job<C>()` which execs `smelt run <path>` and returns the raw exit code with a WARN log on exit 127. Added 3 mock unit tests and 1 gated integration test. Removed `#[allow(dead_code)]` from `key_env` and `port` on WorkerConfig.

## Verification

- `cargo test -p smelt-cli -- ssh::tests::test_deliver_manifest_mock` — 1 test passes ✅
- `cargo test -p smelt-cli -- test_run_remote_job_mock` — 2 tests pass ✅
- `cargo test -p smelt-cli -- test_scp_args` — 2 tests pass ✅
- `cargo test --workspace` — 155 passed, 0 failed ✅
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` — returns 2 (retry_backoff_secs + ssh_timeout_secs)

## Diagnostics

- Exit code 127 from run_remote_job → check serve.log for "smelt may not be on the remote PATH" warnings
- SCP failures → anyhow Err with host, exit code, stderr snippet; also WARN log with same details
- key_env resolved path appears only in DEBUG logs (D112 compliant)

## Deviations

- Plan said to remove `#[allow(dead_code)]` from `ssh_timeout_secs` on ServerConfig, but nothing reads that field yet (consumed in S04 dispatch routing). Kept the annotation to avoid a compiler warning. dead_code count is 2 instead of plan's expected 1.
- T01 work (scp_to, build_scp_args, MockSshClient) was not yet committed; implemented as part of T02 execution.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — Added scp_to() trait method, build_scp_args(), SubprocessSshClient::scp_to(), MockSshClient, deliver_manifest(), run_remote_job(), 5 new unit tests (2 scp args + 3 mock-based)
- `crates/smelt-cli/src/serve/config.rs` — Removed dead_code annotations from key_env and port on WorkerConfig
- `crates/smelt-cli/src/serve/tests.rs` — Added gated test_manifest_delivery_and_remote_exec integration test
