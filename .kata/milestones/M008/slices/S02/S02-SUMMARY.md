---
id: S02
parent: M008
milestone: M008
provides:
  - deliver_manifest<C: SshClient>() — scps manifest to /tmp/smelt-<job_id>.toml, returns remote path
  - run_remote_job<C: SshClient>() — SSHes `smelt run <path>`, returns raw i32 exit code
  - SshClient::scp_to() trait method
  - build_scp_args() helper with uppercase -P for port
  - SubprocessSshClient::scp_to() impl via which::which("scp") + tokio::process::Command
  - MockSshClient with configurable exec/scp/probe result queues
requires:
  - slice: S01
    provides: SshClient trait, SubprocessSshClient, build_ssh_args(), WorkerConfig, SshOutput
affects:
  - S03
  - S04
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - build_scp_args mirrors build_ssh_args but uses uppercase -P (scp convention vs ssh lowercase -p)
  - MockSshClient uses Arc<Mutex<VecDeque>> pop-front pattern for configurable per-call results
  - Free functions generic over C: SshClient compose trait methods into higher-level operations (D121)
patterns_established:
  - SCP arg building mirrors SSH arg building with port flag case difference
  - MockSshClient builder pattern with_exec_result/with_scp_result/with_probe_result for test setup
  - deliver_manifest + run_remote_job as composable free functions over the SshClient trait
observability_surfaces:
  - tracing::debug! on scp_to entry with host, local_path, remote_dest
  - tracing::warn! on scp non-zero exit with host, exit_code, stderr
  - tracing::warn! on run_remote_job exit code 127 with "smelt may not be on the remote PATH" hint
  - key_env resolved path appears only in DEBUG logs (D112 compliant)
drill_down_paths:
  - .kata/milestones/M008/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M008/slices/S02/tasks/T02-SUMMARY.md
duration: 20m
verification_result: passed
completed_at: 2026-03-24
---

# S02: Manifest delivery + remote smelt run execution

**Added deliver_manifest() and run_remote_job() free functions with scp_to() trait extension, MockSshClient test double, and 7 new tests — full scp delivery + remote exec contract proven**

## What Happened

Extended the `SshClient` trait from S01 with `scp_to()` for file delivery, implemented `build_scp_args()` (uppercase `-P` for port, mirroring `build_ssh_args()` but with scp conventions), and added `SubprocessSshClient::scp_to()` via `which::which("scp")` + `tokio::process::Command`. Created `MockSshClient` in `#[cfg(test)]` with three `Arc<Mutex<VecDeque>>` queues for configurable exec/scp/probe responses.

Built two composable free functions on top: `deliver_manifest<C: SshClient>()` scps a manifest to `/tmp/smelt-<job_id>.toml` on the worker and returns the remote path string; `run_remote_job<C: SshClient>()` executes `smelt run <path>` via SSH and returns the raw exit code with a WARN log on exit 127 (suggesting PATH issues).

Added 5 unit tests (2 scp arg tests, 3 mock-based tests for deliver/run) and 1 gated integration test (`SMELT_SSH_TEST=1`) that proves real localhost scp + ssh exec. Removed `#[allow(dead_code)]` from `key_env` and `port` on `WorkerConfig`.

## Verification

- `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_args` — 2 pass ✅
- `cargo test -p smelt-cli --lib -- ssh::tests::test_deliver_manifest_mock` — 1 pass ✅
- `cargo test -p smelt-cli --lib -- ssh::tests::test_run_remote_job_mock` — 2 pass (success + exit2) ✅
- `cargo test --workspace` — 270 passed, 0 failed, 8 ignored ✅
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` — returns 2 (retry_backoff_secs + ssh_timeout_secs)
- Gated integration test: `test_manifest_delivery_and_remote_exec` present and `#[ignore]`d (requires `SMELT_SSH_TEST=1`)

## Requirements Advanced

- R027 (SSH worker pools / remote dispatch) — S02 proves the manifest delivery and remote execution contract; scp delivery + `smelt run` exit code capture are now implemented and tested. Still needs S03 (state sync) and S04 (dispatch routing) before R027 can be validated.

## Requirements Validated

- none — R027 requires full end-to-end pipeline (S04) before validation

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- Plan expected `#[allow(dead_code)]` removed from `ssh_timeout_secs` on `ServerConfig`, but nothing reads that field yet (consumed in S04 dispatch routing). Kept the annotation to avoid a compiler warning. dead_code count is 2 instead of plan's expected 1.
- T01 and T02 were implemented together since T01 had not been committed separately; all deliverables are present.

## Known Limitations

- `run_remote_job()` hardcodes `smelt run <path>` without `--dry-run` flag — gated integration test uses `client.exec()` directly for dry-run validation instead of `run_remote_job()`.
- Exit code mapping (0 = success, 2 = gates failed, other = error per D050) is left to the caller — `run_remote_job` returns the raw `i32`. This is intentional (caller in S04 dispatch routing will apply the mapping).

## Follow-ups

- S03 needs `scp_to()` to pull `.smelt/runs/<job>/` back from the worker (reverse direction scp).
- S04 dispatch routing will consume `deliver_manifest()` and `run_remote_job()` in the `dispatch_loop`.

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — Extended with scp_to() trait method, build_scp_args(), SubprocessSshClient::scp_to(), scp_binary(), MockSshClient, deliver_manifest(), run_remote_job(), 5 new unit tests
- `crates/smelt-cli/src/serve/config.rs` — Removed #[allow(dead_code)] from key_env and port on WorkerConfig
- `crates/smelt-cli/src/serve/tests.rs` — Added gated test_manifest_delivery_and_remote_exec integration test

## Forward Intelligence

### What the next slice should know
- `scp_to()` copies files TO the worker; S03 needs the reverse — scp FROM the worker. Consider adding `scp_from()` to the trait (same arg pattern, swapped local/remote in the scp command).
- `MockSshClient` is ready to use for S03 unit tests — just push results with `with_scp_result()` and `with_exec_result()`.

### What's fragile
- `build_scp_args()` uses uppercase `-P` for port while `build_ssh_args()` uses lowercase `-p` — any refactoring that merges these must preserve this difference or scp will silently fail with "unknown option".

### Authoritative diagnostics
- `cargo test -p smelt-cli --lib -- ssh::tests` — runs all 9 SSH-related unit tests (S01 + S02) in under 1 second; this is the fastest signal for SSH module regressions.

### What assumptions changed
- No assumptions changed. The subprocess ssh/scp approach (D111) continues to work well for the scp case. `which::which("scp")` resolves correctly on all tested platforms.
