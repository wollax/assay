# S01: WorkerConfig + SSH connection proof

**Goal:** Add `[[workers]]` support to `ServerConfig` and prove the SSH subprocess approach by implementing `SshClient` with a `SubprocessSshClient` — exec a command, capture stdout, detect offline host within 3 seconds.
**Demo:** `[[workers]]` entries parse from `server.toml`; a `#[tokio::test]` gated by `SMELT_SSH_TEST=1` connects to localhost SSH, executes `echo hello`, and captures `hello` in stdout; a second gated test connects to a refused port and gets an error back within 3 seconds; `cargo test --workspace` stays green without the env var set.

## Must-Haves

- `WorkerConfig { host: String, user: String, key_env: String, port: u16 }` with `#[derive(Deserialize)]` and `#[serde(deny_unknown_fields)]`
- `ServerConfig::workers: Vec<WorkerConfig>` — parses `[[workers]]` array; defaults to empty vec when absent
- `ServerConfig::ssh_timeout_secs: u64` — defaults to 3; used by `SubprocessSshClient`
- `SshClient` trait with `exec(worker, cmd) -> Result<SshOutput>` and `probe(worker) -> Result<()>`
- `SubprocessSshClient` unit struct implementing `SshClient` via `tokio::process::Command::new("ssh")`
- SSH flags: `-o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=<ssh_timeout_secs> -i <key_path>`
- `SMELT_SSH_TEST=1` gated integration test: connect to localhost, exec `echo hello`, stdout == `"hello"`
- `SMELT_SSH_TEST=1` gated offline test: connect to `127.0.0.1:19222` (refused), error returned in ≤ 3s
- `examples/server.toml` updated with a commented `[[workers]]` block
- `cargo test --workspace` green with and without `SMELT_SSH_TEST=1`

## Proof Level

- This slice proves: contract + integration (localhost SSH path exercised by gated test)
- Real runtime required: yes (SSH to localhost, gated by `SMELT_SSH_TEST=1`)
- Human/UAT required: no (all assertions are automated)

## Verification

- `cargo test --workspace` — zero failures (without `SMELT_SSH_TEST=1`)
- `SMELT_SSH_TEST=1 cargo test -p smelt-cli test_ssh` — both gated tests pass (requires `sshd` on localhost)
- `cargo test -p smelt-cli test_worker_config` — config roundtrip, defaults, deny_unknown_fields, validation
- Manual: `grep -A 10 '\[\[workers\]\]' examples/server.toml` shows a commented example block

## Observability / Diagnostics

- Runtime signals: `SubprocessSshClient::exec()` logs `tracing::debug!("ssh exec: {} {:?}", worker.host, cmd)` on entry; on error logs `tracing::warn!("ssh exec failed: host={} err={}", worker.host, e)`; `probe()` logs warn on failure
- Inspection surfaces: SSH subprocess exit code and stderr propagated into `SmeltError`; error message includes host, cmd, exit code, and captured stderr snippet — enough for a future agent to diagnose connection vs auth vs timeout failures without re-running
- Failure state exposed: `SshOutput { stdout: String, exit_code: i32, stderr: String }` — all fields always populated; callers can inspect `exit_code != 0` and `stderr` to distinguish timeout from auth failure
- Redaction constraints: `key_env` field is an env var *name*, never the key value — never log the resolved key path in INFO/WARN; DEBUG may log the `-i <path>` argument for diagnosis

## Integration Closure

- Upstream surfaces consumed: `crates/smelt-cli/src/serve/config.rs` (ServerConfig), `examples/server.toml`
- New wiring introduced in this slice: `crates/smelt-cli/src/serve/ssh.rs` (new module, `mod ssh;` added to `serve/mod.rs`)
- What remains before the milestone is truly usable end-to-end: S02 (manifest delivery via scp + remote `smelt run` execution), S03 (state sync back), S04 (dispatch routing + round-robin + TUI field)

## Tasks

- [x] **T01: Add WorkerConfig and ssh_timeout_secs to ServerConfig** `est:45m`
  - Why: Delivers the config side of the boundary contract — `[[workers]]` must parse before any SSH code runs; tests prove the contract is locked
  - Files: `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/tests.rs`, `examples/server.toml`
  - Do: Add `WorkerConfig` struct with `deny_unknown_fields`; add `workers: Vec<WorkerConfig>` and `ssh_timeout_secs: u64` (default 3) to `ServerConfig` using `#[serde(default = "...")]` functions; extend `ServerConfig::validate()` to reject empty `host`/`user` on any worker entry; add commented `[[workers]]` block to `examples/server.toml`; add tests to `tests.rs` (config roundtrip with workers, defaults, deny_unknown_fields, validation errors for empty host/user)
  - Verify: `cargo test -p smelt-cli test_worker_config` passes; `cargo test -p smelt-cli test_server_config` still passes
  - Done when: `ServerConfig` with `workers: Vec<WorkerConfig>` round-trips through TOML; empty `host` or `user` in a worker entry fails `validate()`; existing `server.toml` files without `[[workers]]` parse correctly

- [x] **T02: Implement SshClient trait and SubprocessSshClient** `est:1h`
  - Why: Delivers the SSH execution boundary contract — S02 depends on `SshClient`, `SshOutput`, and the fast-fail probe pattern; gated integration tests prove the subprocess approach works end-to-end
  - Files: `crates/smelt-cli/src/serve/ssh.rs` (new), `crates/smelt-cli/src/serve/mod.rs`
  - Do: Create `ssh.rs`; define `SshOutput { stdout: String, stderr: String, exit_code: i32 }`; define `SshClient` trait with `async fn exec(&self, worker: &WorkerConfig, timeout_secs: u64, cmd: &str) -> Result<SshOutput>` and `async fn probe(&self, worker: &WorkerConfig, timeout_secs: u64) -> Result<()>`; implement `SubprocessSshClient` (unit struct) using `tokio::process::Command::new("ssh")` with flags `-o BatchMode=yes -o StrictHostKeyChecking=accept-new -o ConnectTimeout=<secs> -i <key_path>` (key path resolved from `std::env::var(worker.key_env)` — warn on missing but allow fallback to default key); `probe()` runs `ssh ... echo smelt-probe` and maps non-zero exit to error; add `mod ssh;` to `serve/mod.rs`; add inline tests: one unit test asserting the `ssh` args vector is built correctly (no actual SSH needed); two `SMELT_SSH_TEST=1` gated `#[tokio::test] #[ignore]` tests — `test_ssh_exec_localhost` (exec `echo hello`, assert stdout == `"hello"`, exit_code == 0) and `test_ssh_probe_offline` (probe `127.0.0.1:19222`, assert error returned within 4s)
  - Verify: `cargo test --workspace` green; `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_ssh` passes on a machine with localhost sshd
  - Done when: `SubprocessSshClient::exec()` returns `SshOutput` with stdout captured; `SubprocessSshClient::probe()` returns `Err` within ≤ 3s when the target port is closed; `cargo test --workspace` passes without `SMELT_SSH_TEST=1`

## Files Likely Touched

- `crates/smelt-cli/src/serve/config.rs`
- `crates/smelt-cli/src/serve/ssh.rs` (new)
- `crates/smelt-cli/src/serve/mod.rs`
- `crates/smelt-cli/src/serve/tests.rs`
- `examples/server.toml`
