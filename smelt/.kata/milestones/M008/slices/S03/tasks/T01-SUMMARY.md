---
id: T01
parent: S03
milestone: M008
provides:
  - scp_from() method on SshClient trait for recursive remote-to-local copy
  - SubprocessSshClient::scp_from() implementation using -r flag and remote-first arg order
  - MockSshClient::scp_from_results separate queue with with_scp_from_result() builder
  - 3 unit tests covering args verification, mock success, and mock failure paths
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
key_decisions:
  - "scp_from uses -r flag unconditionally since the primary use case is copying run state directories"
patterns_established:
  - "scp_from mirrors scp_to pattern: same build_scp_args helper, same tracing debug/warn, same error shape"
  - "MockSshClient uses separate result queues per method (scp_results vs scp_from_results) for independent test configuration"
observability_surfaces:
  - "tracing::debug! on scp_from entry with host, remote_src, local_dest"
  - "tracing::warn! on non-zero exit with host, exit_code, stderr"
  - "anyhow::Error includes host, exit_code, and stderr content for caller inspection"
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Add scp_from() to SshClient trait with recursive copy and MockSshClient extension

**Extended SshClient trait with scp_from() for recursive remote→local directory copy, implemented in SubprocessSshClient and MockSshClient with 3 passing unit tests**

## What Happened

Added `scp_from()` to the `SshClient` trait with the signature `(&self, worker, timeout_secs, remote_src, local_dest) -> Result<()>`. The `SubprocessSshClient` implementation builds a `user@host:/path` remote spec, passes `-r` for recursive copy to `build_scp_args()`, and follows the same debug/warn tracing pattern as `scp_to()`. The `MockSshClient` got a dedicated `scp_from_results: Arc<Mutex<VecDeque<Result<()>>>>` queue with a `with_scp_from_result()` builder, keeping it independent from the existing `scp_results` queue used by `scp_to`.

Three unit tests added: `test_scp_from_args_recursive` validates `-r` flag presence, remote-before-local ordering, and uppercase `-P` for custom port; `test_scp_from_mock_success` and `test_scp_from_mock_failure` validate the mock queue behavior for both paths.

## Verification

- `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_from` — 3/3 passed (args_recursive, mock_success, mock_failure)
- `cargo test -p smelt-cli --lib` — 68 passed, 0 failed, 3 ignored (gated SSH tests)
- `cargo test --workspace` — all lib/core tests pass; 1 pre-existing docker_lifecycle test failure (`test_cli_run_invalid_manifest`) unrelated to this change

## Diagnostics

- grep logs for `scp_from entry` to see invocations (host, remote_src, local_dest)
- grep logs for `scp_from non-zero exit` to see failures (host, exit_code, stderr)
- Error messages include host, exit code, and stderr content for programmatic inspection

## Deviations

None.

## Known Issues

- Pre-existing failure in `docker_lifecycle::test_cli_run_invalid_manifest` — unrelated to scp_from changes

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — Added scp_from() to SshClient trait, SubprocessSshClient impl, MockSshClient scp_from_results queue + builder, 3 unit tests
