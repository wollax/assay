---
estimated_steps: 5
estimated_files: 1
---

# T01: Extend SshClient with scp_to + build_scp_args + MockSshClient

**Slice:** S02 — Manifest delivery + remote smelt run execution
**Milestone:** M008

## Description

Extend the `SshClient` trait with an `async fn scp_to()` method for copying files to remote workers. Implement `build_scp_args()` as a public helper that correctly uses uppercase `-P` for port (the most common scp bug). Implement `scp_to()` on `SubprocessSshClient` using `which::which("scp")` and `tokio::process::Command`. Create a `MockSshClient` under `#[cfg(test)]` that implements all three trait methods with configurable responses via `Arc<Mutex<VecDeque>>`, enabling T02's mock-based unit tests.

## Steps

1. Add `scp_to()` async method to the `SshClient` trait in ssh.rs:
   - Signature: `async fn scp_to(&self, worker: &WorkerConfig, timeout_secs: u64, local_path: &std::path::Path, remote_dest: &str) -> anyhow::Result<()>`
   - The `remote_dest` is the full `user@host:/path` string — caller assembles it

2. Add `pub fn build_scp_args()` below the existing `build_ssh_args()`:
   - Same common flags: `BatchMode=yes`, `StrictHostKeyChecking=accept-new`, `ConnectTimeout=<N>`
   - **Uppercase `-P`** for port (not lowercase `-p` which means "preserve timestamps" in scp)
   - `-i <key_path>` when key_env resolves, same WARN pattern as build_ssh_args
   - `extra_args` appended at the end (local_path, remote_dest)

3. Implement `scp_to()` on `SubprocessSshClient`:
   - Add `fn scp_binary() -> anyhow::Result<PathBuf>` using `which::which("scp")`
   - Build args via `build_scp_args()`, adding `[local_path.to_str(), remote_dest]` as extra args
   - `tracing::debug!` on entry; `tracing::warn!` on non-zero exit code with stderr
   - Return `Err` on non-zero exit code (scp failure = delivery failure)

4. Create `MockSshClient` in `#[cfg(test)]` block:
   - Fields: `exec_results: Arc<Mutex<VecDeque<anyhow::Result<SshOutput>>>>`, `scp_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>`, `probe_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>`
   - Implement `SshClient` — each method pops from its respective VecDeque; panics if empty (test setup error)
   - Add a `MockSshClient::new()` that takes no args (empty queues) and setter methods `with_exec_result`, `with_scp_result`, `with_probe_result`

5. Add two unit tests:
   - `test_scp_args_build`: default port, verify uppercase `-P` is NOT present (port 22), verify common flags
   - `test_scp_args_custom_port`: port 2222, verify `-P 2222` present (uppercase)

## Must-Haves

- [ ] `scp_to()` method on `SshClient` trait
- [ ] `build_scp_args()` uses uppercase `-P` for port
- [ ] `SubprocessSshClient::scp_to()` uses `which::which("scp")` + `tokio::process::Command`
- [ ] `MockSshClient` with configurable exec/scp/probe results
- [ ] `test_scp_args_build` passes
- [ ] `test_scp_args_custom_port` passes

## Verification

- `cargo test -p smelt-cli -- test_scp_args_build` — passes
- `cargo test -p smelt-cli -- test_scp_args_custom_port` — passes
- `cargo test --workspace` — all existing tests still pass (no regressions)

## Observability Impact

- Signals added/changed: `tracing::debug!` on scp_to entry with host, local_path, remote_dest; `tracing::warn!` on scp non-zero exit with host, exit_code, stderr
- How a future agent inspects this: check `serve.log` for scp-related WARN entries; `SshOutput`-style error details in anyhow error chain
- Failure state exposed: scp failure returns Err with host, exit code, stderr snippet — same pattern as exec()

## Inputs

- `crates/smelt-cli/src/serve/ssh.rs` — S01's SshClient trait, SubprocessSshClient, build_ssh_args() pattern
- `crates/smelt-cli/src/serve/config.rs` — WorkerConfig struct
- S01-SUMMARY forward intelligence: build_scp_args mirrors build_ssh_args; key_env resolution same pattern; uppercase -P for scp port

## Expected Output

- `crates/smelt-cli/src/serve/ssh.rs` — extended with scp_to() trait method, build_scp_args() helper, SubprocessSshClient::scp_to() impl, MockSshClient, 2 new unit tests
