---
id: T01
parent: S02
milestone: M008
provides:
  - SshClient::scp_to() trait method
  - build_scp_args() helper with uppercase -P for port
  - SubprocessSshClient::scp_to() impl using which::which("scp")
  - MockSshClient with configurable exec/scp/probe result queues
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
key_decisions:
  - build_scp_args mirrors build_ssh_args pattern but uses uppercase -P (scp convention vs ssh lowercase -p)
  - MockSshClient uses Arc<Mutex<VecDeque>> pop-front pattern for configurable per-call results
patterns_established:
  - SCP arg building mirrors SSH arg building with port flag case difference
  - MockSshClient builder pattern with_exec_result/with_scp_result/with_probe_result for test setup
observability_surfaces:
  - tracing::debug! on scp_to entry with host, local_path, remote_dest
  - tracing::warn! on scp non-zero exit with host, exit_code, stderr
duration: 5m
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T01: Extend SshClient with scp_to + build_scp_args + MockSshClient

**Added scp_to() trait method, build_scp_args() with uppercase -P port flag, SubprocessSshClient::scp_to() impl, and MockSshClient test double**

## What Happened

All T01 deliverables were implemented in ssh.rs as part of the S02 implementation:

1. **scp_to() trait method** added to `SshClient` with signature matching the plan exactly — takes worker, timeout, local_path, remote_dest.
2. **build_scp_args()** implemented as a public method on `SubprocessSshClient`, mirroring `build_ssh_args()` but using uppercase `-P` for port (SCP convention). Same common flags: BatchMode=yes, StrictHostKeyChecking=accept-new, ConnectTimeout. Same key_env resolution with WARN on missing/empty.
3. **SubprocessSshClient::scp_to()** uses `scp_binary()` (which::which("scp")), builds args via build_scp_args, debug log on entry, warn + Err on non-zero exit.
4. **MockSshClient** in `#[cfg(test)]` block with three `Arc<Mutex<VecDeque>>` fields for exec/probe/scp results. Builder methods `with_exec_result`, `with_scp_result`, `with_probe_result`. Panics replaced with descriptive Err on empty queue.
5. **Two unit tests**: `test_scp_args_build` (default port, no -P) and `test_scp_args_custom_port` (port 2222, uppercase -P 2222, no lowercase -p).

## Verification

- `cargo test -p smelt-cli --lib -- scp_args` — 2 tests pass (test_scp_args_build, test_scp_args_custom_port)
- `cargo test --workspace` — 155 passed, 0 failed, no regressions

### Slice-level checks (partial — T01 is intermediate):
- ✅ `cargo test -p smelt-cli -- test_scp_args` — 2 scp arg unit tests pass
- ✅ `cargo test -p smelt-cli -- test_deliver_manifest_mock` — passes (implemented alongside)
- ✅ `cargo test -p smelt-cli -- test_run_remote_job_mock` — 2 mock run tests pass
- ✅ `cargo test --workspace` — all tests pass, 0 failures
- ⬜ Integration test (manual, gated by SMELT_SSH_TEST=1)

## Diagnostics

- SCP operations emit `tracing::debug!` on entry with host/local_path/remote_dest
- SCP failures emit `tracing::warn!` with host/exit_code/stderr, then return anyhow::Err with same details
- Key_env resolution follows same DEBUG/WARN pattern as SSH — resolved path only in DEBUG, never INFO/WARN (D112)

## Deviations

None. Implementation matches plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — Extended with scp_to() trait method, build_scp_args(), SubprocessSshClient::scp_to(), scp_binary(), MockSshClient, and 2 scp args unit tests
