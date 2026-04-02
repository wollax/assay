# S02: Manifest delivery + remote smelt run execution

**Goal:** Given a manifest TOML path and a WorkerConfig, `deliver_manifest` scps the file to `/tmp/smelt-<job_id>.toml` on the worker and `run_remote_job` SSHes `smelt run <path>`, returning the raw exit code. Both functions are generic over `C: SshClient` and fully testable via MockSshClient without real SSH.
**Demo:** `cargo test -p smelt-cli` runs 7+ new tests (scp args, mock deliver, mock run success/exit2, scp args custom port) all green; gated integration test proves real localhost scp + ssh exec path.

## Must-Haves

- `scp_to()` method added to `SshClient` trait
- `build_scp_args()` helper using uppercase `-P` for port (not lowercase `-p`)
- `SubprocessSshClient::scp_to()` implemented via `tokio::process::Command` + `which::which("scp")`
- `deliver_manifest<C: SshClient>()` free function — scps manifest to `/tmp/smelt-<job_id>.toml`, returns the remote path string
- `run_remote_job<C: SshClient>()` free function — SSHes `smelt run <path>`, returns raw `i32` exit code (caller maps 0/2/other per D050)
- `MockSshClient` with configurable responses for all three trait methods (`exec`, `probe`, `scp_to`)
- `#[allow(dead_code)]` removed from `key_env`, `port`, `ssh_timeout_secs` in `config.rs`
- Unit tests: `test_scp_args_build`, `test_scp_args_custom_port`, `test_deliver_manifest_mock`, `test_run_remote_job_mock_success`, `test_run_remote_job_mock_exit2`
- Gated integration test: `test_manifest_delivery_and_remote_exec` (`SMELT_SSH_TEST=1`)
- `cargo test --workspace` all green, zero regressions

## Proof Level

- This slice proves: contract + integration (localhost SSH)
- Real runtime required: yes (for gated integration test only; mock tests run without SSH)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli -- test_scp_args` — 2 scp arg unit tests pass
- `cargo test -p smelt-cli -- test_deliver_manifest_mock` — mock deliver test passes
- `cargo test -p smelt-cli -- test_run_remote_job_mock` — 2 mock run tests pass (success + exit2)
- `cargo test --workspace` — all tests pass, 0 failures, no regressions
- `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_manifest_delivery` — gated integration test passes (manual verification)
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` returns 1 (only `retry_backoff_secs` remains)

## Observability / Diagnostics

- Runtime signals: `tracing::debug!` on scp_to entry with host/local_path/remote_dest; `tracing::warn!` on scp non-zero exit with host/exit_code/stderr; `tracing::warn!` on run_remote_job exit code 127 ("command not found" hint for PATH issues)
- Inspection surfaces: `SshOutput` struct returned from all operations with stdout/stderr/exit_code fully populated
- Failure visibility: scp failure → anyhow error with host, exit code, stderr snippet; exit code 127 on run_remote_job → WARN log suggesting smelt not on remote PATH
- Redaction constraints: key_env resolved path appears only in DEBUG logs (D112); never at INFO/WARN

## Integration Closure

- Upstream surfaces consumed: `SshClient` trait, `SubprocessSshClient`, `build_ssh_args()`, `WorkerConfig`, `SshOutput` (all from S01 ssh.rs); `JobId` with `Display` (from types.rs)
- New wiring introduced in this slice: `scp_to()` trait method, `build_scp_args()`, `deliver_manifest()`, `run_remote_job()` — all in ssh.rs; no dispatch wiring (that's S04)
- What remains before the milestone is truly usable end-to-end: S03 (state sync back via scp), S04 (dispatch routing + round-robin + TUI/API worker field)

## Tasks

- [x] **T01: Extend SshClient with scp_to + build_scp_args + MockSshClient** `est:30m`
  - Why: The SshClient trait needs an scp_to() method for file delivery, build_scp_args() for correct scp flag assembly (uppercase -P), and a MockSshClient to enable unit testing of deliver_manifest/run_remote_job without real SSH
  - Files: `crates/smelt-cli/src/serve/ssh.rs`
  - Do: Add `scp_to()` to `SshClient` trait; implement `build_scp_args()` as pub fn (uppercase `-P` for port); implement `scp_to()` on `SubprocessSshClient` via `which::which("scp")` + `tokio::process::Command`; create `MockSshClient` in `#[cfg(test)]` with `Arc<Mutex<VecDeque>>` for configurable responses; add `test_scp_args_build` and `test_scp_args_custom_port` unit tests
  - Verify: `cargo test -p smelt-cli -- test_scp_args` — both pass
  - Done when: `scp_to()` on trait + SubprocessSshClient, `build_scp_args()` pub, MockSshClient created, 2 scp arg tests green

- [x] **T02: Add deliver_manifest + run_remote_job + integration test + remove dead_code** `est:35m`
  - Why: These are the two free functions S02 produces for the boundary map — they compose scp_to/exec into manifest delivery and remote job execution; mock tests prove contract; integration test proves real SSH path; dead_code cleanup resolves S01 follow-up
  - Files: `crates/smelt-cli/src/serve/ssh.rs`, `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/tests.rs`
  - Do: Add `deliver_manifest<C: SshClient>()` and `run_remote_job<C: SshClient>()` free fns in ssh.rs; add exit-code-127 WARN log in run_remote_job; add 3 mock-based unit tests in ssh.rs (deliver_manifest_mock, run_remote_job_mock_success, run_remote_job_mock_exit2); add gated `test_manifest_delivery_and_remote_exec` integration test in tests.rs using `--dry-run`; remove `#[allow(dead_code)]` from key_env, port, ssh_timeout_secs in config.rs
  - Verify: `cargo test --workspace` — all pass, 0 failures
  - Done when: deliver_manifest + run_remote_job implemented; 3 mock tests + 1 gated integration test added; dead_code annotations removed; workspace tests green

## Files Likely Touched

- `crates/smelt-cli/src/serve/ssh.rs`
- `crates/smelt-cli/src/serve/config.rs`
- `crates/smelt-cli/src/serve/tests.rs`
