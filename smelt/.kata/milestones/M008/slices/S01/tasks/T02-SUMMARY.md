---
id: T02
parent: S01
milestone: M008
provides:
  - SshOutput { stdout: String, stderr: String, exit_code: i32 } — public output struct
  - SshClient trait with async exec() and async probe() — not object-safe, use generics per D060
  - SubprocessSshClient unit struct — SshClient impl via tokio::process::Command + which::which("ssh")
  - build_ssh_args() — assembles BatchMode=yes, StrictHostKeyChecking=accept-new, ConnectTimeout=<N>, -p <port> if non-22, -i <key_path> if key_env resolves
  - probe() — wraps exec("echo smelt-probe"), returns Err on non-zero exit; fast-fail via SSH's own ConnectTimeout (no zombie processes)
  - mod ssh + pub re-exports in serve/mod.rs — SshClient, SshOutput, SubprocessSshClient accessible from serve::
  - test_ssh_args_build and test_ssh_args_build_custom_port — non-gated unit tests, cargo test --workspace green
  - test_ssh_exec_localhost and test_ssh_probe_offline — gated #[ignore] tests, require SMELT_SSH_TEST=1 --include-ignored
requires: []
affects: [S02, S03, S04]
key_files:
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "D019 pattern applied: #[allow(async_fn_in_trait)] on SshClient; async fn acceptable since trait is crate-internal and Send bounds not required by current callsites"
  - "probe() delegates fast-fail entirely to SSH's -o ConnectTimeout — no tokio::time::timeout wrapper, no zombie subprocess risk (D111)"
  - "key_env missing → WARN log + SSH falls back to default key (no hard error); allows dev without explicit key_env setup"
  - "build_ssh_args() is pub for testability — lets unit tests assert arg composition without any real SSH call"
patterns_established:
  - "SshClient trait pattern: #[allow(async_fn_in_trait)] + SubprocessSshClient unit struct — same shape as GitCli implements GitOps"
  - "Gated integration test guard: if std::env::var(SMELT_SSH_TEST).is_err() { return; } at top of #[tokio::test] #[ignore]"
drill_down_paths:
  - .kata/milestones/M008/slices/S01/tasks/T02-PLAN.md
duration: 30min
verification_result: pass
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Implement SshClient trait and SubprocessSshClient

**`SshClient` trait + `SubprocessSshClient` impl via `tokio::process::Command`; SSH flags assembled by `build_ssh_args()`; offline fast-fail via `-o ConnectTimeout`; 4 tests (2 non-gated passing, 2 gated ignored).**

## What Happened

Created `crates/smelt-cli/src/serve/ssh.rs` with the full SSH abstraction layer:

- `SshOutput` struct captures `stdout`, `stderr`, and `exit_code` (mapped to `-1` on signal kill).
- `SshClient` trait uses `async fn` (RPITIT pattern per D019) with `#[allow(async_fn_in_trait)]` to suppress the lint; trait is crate-internal and generics-based per D060.
- `SubprocessSshClient` locates the SSH binary via `which::which("ssh")` (not hardcoded path). Its private `build_ssh_args()` is `pub` for unit testability.
- `exec()` logs `tracing::debug!` on entry (host + cmd + args) and `tracing::warn!` on non-zero exit (host + exit_code + stderr).
- `probe()` calls `exec("echo smelt-probe")` and maps the result; fast-fail is entirely delegated to SSH's own `-o ConnectTimeout` flag — no `tokio::time::timeout` wrapper needed, no zombie subprocess risk.
- When `key_env` env var is not set, the implementation logs `tracing::warn!` and lets SSH fall back to default keys — consistent with D112 (key_env is a name, not a value).
- `mod ssh;` + `pub use ssh::{SshClient, SshOutput, SubprocessSshClient};` added to `serve/mod.rs`.

## Deviations

None — implementation follows the plan exactly.

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — new file: SshOutput, SshClient trait, SubprocessSshClient impl, 2 non-gated unit tests, 2 gated integration tests
- `crates/smelt-cli/src/serve/mod.rs` — added `pub mod ssh;` and `pub use ssh::*` re-exports

## Verification Results

| Check | Status | Evidence |
|---|---|---|
| `cargo test --workspace` | ✅ PASS | 60 passed, 0 failed, 2 ignored (gated SSH tests) |
| `test_ssh_args_build` | ✅ PASS | Asserts BatchMode=yes, StrictHostKeyChecking=accept-new, ConnectTimeout=3, user@host |
| `test_ssh_args_build_custom_port` | ✅ PASS | Asserts -p 2222 present for non-default port |
| `test_ssh_exec_localhost` | ⏭ GATED | Requires SMELT_SSH_TEST=1 --include-ignored |
| `test_ssh_probe_offline` | ⏭ GATED | Requires SMELT_SSH_TEST=1 --include-ignored |
