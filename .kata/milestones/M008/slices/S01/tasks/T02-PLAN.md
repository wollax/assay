---
estimated_steps: 6
estimated_files: 3
---

# T02: Implement SshClient trait and SubprocessSshClient

**Slice:** S01 — WorkerConfig + SSH connection proof
**Milestone:** M008

## Description

Create `crates/smelt-cli/src/serve/ssh.rs` with the `SshClient` trait and its `SubprocessSshClient` implementation. This retires the two highest-risk unknowns from the M008 proof strategy:

1. **SSH execution approach** — proven by executing `echo hello` on localhost and capturing stdout
2. **Offline-worker fast-fail** — proven by connecting to a refused port and getting an error within ≤ 3s

Key constraints from research (D111, S01-RESEARCH.md):
- Use `tokio::process::Command` (not `std::process::Command`) — dispatch loop is async
- SSH flags: `-o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=<N>`
- Key path resolved from `std::env::var(&worker.key_env)` at call time; if missing, warn and omit `-i` flag (SSH falls back to default key)
- Offline fast-fail implemented via SSH's own `-o ConnectTimeout` — NOT via `tokio::time::timeout` which would leave the subprocess running
- `SshClient` trait: generic `<C: SshClient>` at callsites (D060 pattern) — avoid `dyn SshClient` / object safety issues with `async fn`
- Gated tests use `#[tokio::test] #[ignore]` + `SMELT_SSH_TEST=1` env var guard (D089 pattern)
- Use `which::which("ssh")` for the binary path — do not hardcode `/usr/bin/ssh`

## Steps

1. Create `crates/smelt-cli/src/serve/ssh.rs`. Define:
   ```rust
   pub struct SshOutput {
       pub stdout: String,
       pub stderr: String,
       pub exit_code: i32,
   }
   ```

2. Define the `SshClient` trait (not object-safe — uses RPITIT/async fn per D019):
   ```rust
   pub trait SshClient {
       async fn exec(&self, worker: &WorkerConfig, timeout_secs: u64, cmd: &str)
           -> anyhow::Result<SshOutput>;
       async fn probe(&self, worker: &WorkerConfig, timeout_secs: u64)
           -> anyhow::Result<()>;
   }
   ```

3. Implement `SubprocessSshClient` (unit struct):
   - `build_ssh_args(worker, timeout_secs, extra_args)` private helper that assembles the common flag vector: `-o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=<N> [-p <port> if port != 22] [-i <key_path> if key_env resolves]`; logs `tracing::debug!` with host and args (excluding key path content)
   - `exec()`: calls `Command::new(ssh_binary).args(&[...common flags..., "<user>@<host>", cmd]).output().await`; collects stdout/stderr as strings; returns `SshOutput` with exit_code (map `None` exit status to `-1`); on `exit_code != 0` also logs `tracing::warn!`
   - `probe()`: calls `exec(worker, timeout_secs, "echo smelt-probe")`; maps `Ok` result with `exit_code == 0` to `Ok(())`; any non-zero exit or error maps to `Err(anyhow!("ssh probe failed: ..."))`

4. Add `mod ssh; pub use ssh::{SshClient, SshOutput, SubprocessSshClient};` in `crates/smelt-cli/src/serve/mod.rs`.

5. Add a non-gated unit test in `ssh.rs` (`#[cfg(test)]` block): `test_ssh_args_build` — create a `WorkerConfig` with known values, call `SubprocessSshClient::build_ssh_args()`, and assert the returned args contain `BatchMode=yes`, `StrictHostKeyChecking=accept-new`, `ConnectTimeout=3`, and `user@host`. No actual SSH call — pure arg inspection. This test runs in `cargo test --workspace` without any env var gate.

6. Add two gated integration tests in `crates/smelt-cli/src/serve/tests.rs` (or inline in `ssh.rs` `#[cfg(test)]`):
   - `test_ssh_exec_localhost`: guarded by `if std::env::var("SMELT_SSH_TEST").is_err() { return; }` at top; create a `WorkerConfig` targeting localhost; call `SubprocessSshClient.exec(&worker, 5, "echo hello")`; assert `output.exit_code == 0` and `output.stdout.trim() == "hello"`
   - `test_ssh_probe_offline`: guarded by same env var check; create `WorkerConfig { host: "127.0.0.1", port: 19222, user: "nobody", key_env: "SMELT_SSH_KEY_UNUSED" }`; record `Instant::now()`; call `SubprocessSshClient.probe(&worker, 3)`; assert result is `Err`; assert elapsed < 4s

   Both tests use `#[tokio::test] #[ignore]` — the `--include-ignored` flag is required to run them.

## Must-Haves

- [ ] `SshOutput { stdout, stderr, exit_code }` struct defined and `pub`
- [ ] `SshClient` trait with `exec()` and `probe()` async methods
- [ ] `SubprocessSshClient` implements `SshClient` via `tokio::process::Command`
- [ ] SSH flags include `BatchMode=yes`, `StrictHostKeyChecking=accept-new`, `ConnectTimeout=<N>`
- [ ] `probe()` returns `Err` within ≤ 3s when target port is refused (proven by `test_ssh_probe_offline`)
- [ ] `exec()` captures stdout correctly (proven by `test_ssh_exec_localhost`)
- [ ] `mod ssh;` registered in `serve/mod.rs`
- [ ] `test_ssh_args_build` passes in `cargo test --workspace` without `SMELT_SSH_TEST`
- [ ] `cargo test --workspace` green without `SMELT_SSH_TEST=1`
- [ ] With `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_ssh`: both gated tests pass

## Verification

- `cargo test --workspace` — all tests pass; no new failures
- `cargo test -p smelt-cli test_ssh_args_build` — passes without `SMELT_SSH_TEST`
- `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_ssh_exec_localhost` — passes on machine with localhost sshd
- `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_ssh_probe_offline` — returns Err within 4s

## Observability Impact

- Signals added/changed: `tracing::debug!` on every SSH exec entry (host, command); `tracing::warn!` on SSH exec failure (host, exit code, stderr snippet); `tracing::warn!` when `key_env` is set but env var is missing
- How a future agent inspects this: Run with `RUST_LOG=smelt_cli=debug` to see all SSH calls; error messages include host, exit_code, and stderr content — enough to distinguish timeout (exit_code from SSH's own ConnectTimeout behaviour) from auth failure (stderr contains "Permission denied") from command error
- Failure state exposed: `SshOutput::exit_code` and `SshOutput::stderr` always populated; SSH's `-o ConnectTimeout` ensures the subprocess self-terminates on timeout so no zombie processes

## Inputs

- `crates/smelt-cli/src/serve/config.rs` — `WorkerConfig` struct (from T01)
- `crates/smelt-core/src/git/cli.rs` — `tokio::process::Command` subprocess pattern to replicate
- `crates/smelt-core/src/compose.rs` — `Command::new(...).output().await` + `!output.status.success()` error pattern
- S01-RESEARCH.md — SSH flags, `which::which("ssh")`, `SMELT_SSH_TEST=1` pattern, trait object-safety tradeoffs

## Expected Output

- `crates/smelt-cli/src/serve/ssh.rs` — new file: `SshOutput`, `SshClient` trait, `SubprocessSshClient` impl, `test_ssh_args_build` unit test, two gated `#[ignore]` integration tests
- `crates/smelt-cli/src/serve/mod.rs` — `mod ssh;` + re-exports added
- `crates/smelt-cli/src/serve/tests.rs` — optionally hosts the gated SSH integration tests if preferred over inline; either location is acceptable
