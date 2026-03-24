# M008/S02 — Research

**Date:** 2026-03-24

## Summary

S02 builds directly on S01's `SshClient` trait and `SubprocessSshClient` to implement
two new operations: `deliver_manifest` (scp manifest TOML to `/tmp/smelt-<job_id>.toml`
on the worker) and `run_remote_job` (SSH exec `smelt run <path>` and return the raw exit
code). Both functions should be generic over `C: SshClient` to stay testable without a
real SSH daemon.

The cleanest implementation extends the `SshClient` trait with an `async fn scp_to()`
method alongside the existing `exec()` and `probe()`. This allows `deliver_manifest` to
be a free function that calls `client.scp_to(...)`, and a `MockSshClient` can implement
all three methods for unit tests without spawning any subprocesses.

`SubprocessSshClient` implements `scp_to()` by calling `scp` via
`tokio::process::Command`. A new `build_scp_args()` helper mirrors `build_ssh_args()` in
structure — except `scp` uses `-P` (capital) for port, not `-p` (lowercase). The
integration test is gated by `SMELT_SSH_TEST=1`, uses `--dry-run` to avoid needing Docker
on the test host, and follows the existing `#[tokio::test] #[ignore]` pattern from S01.

## Recommendation

**Extend `SshClient` trait with `scp_to()` + implement `build_scp_args()` + free functions
`deliver_manifest<C: SshClient>` and `run_remote_job<C: SshClient>` all in `ssh.rs`.**

This keeps the abstraction self-contained, enables full mock-based unit testing of both
functions, and is consistent with the trait-extension pattern already established by S01.
The integration test with `SMELT_SSH_TEST=1` proves the full scp + ssh exec path end-to-end.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Finding `scp` binary | `which::which("scp")` | Already used for `ssh`; workspace dep; consistent |
| SSH arg construction | `SubprocessSshClient::build_ssh_args()` | Reuse pattern; `scp_args` share flags |
| Subprocess execution | `tokio::process::Command` | Already used in `exec()`; avoids blocking the async runtime |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/ssh.rs` — `SshClient` trait (`exec`, `probe`), `SubprocessSshClient`, `build_ssh_args()`. **Extend with `scp_to()` method and `build_scp_args()` helper. Add `deliver_manifest` and `run_remote_job` as free fns at bottom of file.**
- `crates/smelt-cli/src/serve/config.rs` — `WorkerConfig { host, user, key_env, port }`. S02 should remove the `#[allow(dead_code)]` annotations on `key_env`, `port`, and `ssh_timeout_secs` since they are now fully consumed.
- `crates/smelt-cli/src/serve/types.rs` — `JobId` implements `Display` as `job-N`. Used directly in remote path: `/tmp/smelt-{job_id}.toml`.
- `crates/smelt-cli/src/serve/dispatch.rs` — `run_job_task` calls `run_with_cancellation()` locally. S04 will add a branch here that calls `deliver_manifest` + `run_remote_job` instead. S02 only needs to expose the two functions; dispatch wiring is S04's job.
- `crates/smelt-cli/src/serve/tests.rs` — Integration test file for `smelt-cli`. New `test_manifest_delivery_and_remote_exec` goes here, not in `ssh.rs`, per the existing pattern of gated tests living in `tests.rs`.

## Constraints

- D111 is firm: subprocess `scp`/`ssh`, no Rust SSH library crates.
- D121 is firm: generic `<C: SshClient>` at callsites, not `dyn SshClient`.
- `scp` uses **`-P`** (capital) for port — different from `ssh`'s `-p` (lowercase). A `build_scp_args()` helper must use `-P`. This is the most common scp bug.
- SSH preserves the remote process's exit code with `BatchMode=yes`: `ssh -o BatchMode=yes user@host smelt run /tmp/x.toml` exits with the same code as `smelt run`. This means exit code 2 propagates correctly.
- The `SMELT_SSH_TEST=1` integration test uses `smelt run --dry-run` to avoid needing Docker on the test host — the exact same reason the existing test uses `--dry-run`.
- `smelt` must be on the non-interactive shell PATH of the remote user. Non-interactive SSH sessions do NOT source `~/.bashrc` or `~/.profile` on most Linux distros. The operator must ensure `smelt` is in a PATH entry loaded by non-interactive shells (e.g. `/usr/local/bin`) or set `PATH` in `~/.ssh/environment` (requires `PermitUserEnvironment yes` in `sshd_config`). Document this limitation.
- `which::which("scp")` already has a `dep` in `smelt-cli/Cargo.toml` via `which.workspace = true` — no new dependency needed.
- No `dead_code` annotations should remain in `config.rs` after S02 — `key_env`, `port`, `ssh_timeout_secs` are all consumed.

## Common Pitfalls

- **`-p` vs `-P` for port** — `ssh` uses `-p`, `scp` uses `-P`. Using lowercase `-p` for scp silently does nothing (or matches scp's `-p` = preserve timestamps). Always use uppercase `-P` in `build_scp_args()`.
- **scp destination format** — Use `user@host:/remote/path`, not `user@host: /remote/path` (no space). The colon+path must be contiguous.
- **Non-interactive PATH** — `smelt run` will fail with "command not found" on the remote if `smelt` is not in the non-interactive shell PATH. The integration test surfaces this immediately; the fix is operator responsibility (not smelt's). Add a clear WARN log when the SSH exec exits with 127.
- **`run_remote_job` returns raw exit code** — Do NOT map exit codes inside `run_remote_job`. Return the raw i32. The dispatch layer (S04) maps 0 → success, 2 → GatesFailed, other non-zero → failure. This preserves the same exit-code semantics as local `smelt run` (D050).
- **`scp_to()` signature** — Takes `local_path: &Path` and `remote_dest: &str`. The remote dest is assembled by the caller: `format!("{}@{}:{}", worker.user, worker.host, remote_path)`. Keep the trait method general; the path construction stays in `deliver_manifest`.
- **Temp file already exists** — `/tmp/smelt-<job_id>.toml` is safe because `JobId` includes a monotonic counter; two concurrent jobs on the same worker get distinct IDs. Not a concern for M008.

## Open Risks

- **`scp` not on PATH in minimal environments** — `which::which("scp")` will fail cleanly; the error message is clear. Low risk for a `smelt serve` daemon (usually runs on a developer workstation).
- **`smelt run --dry-run` PATH issue in integration test** — If the test machine doesn't have `smelt` on the non-interactive PATH for localhost SSH, the integration test will fail with exit code 127. The test should assert `exit_code == 0` AND `stderr` does not contain "not found" to produce a clear diagnostic.
- **scp to `/tmp` succeeds but SSH exec can't find the file** — extremely unlikely on a single-node test, but possible if the SSH session runs as a different user. Not a risk for `SMELT_SSH_TEST=1` tests where the user is the same.

## Implementation Sketch

```
// In ssh.rs — extend the trait
pub trait SshClient {
    async fn exec(...) -> Result<SshOutput>;      // from S01
    async fn probe(...) -> Result<()>;             // from S01
    async fn scp_to(                               // NEW in S02
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        local_path: &std::path::Path,
        remote_dest: &str,                         // "user@host:/tmp/smelt-job-1.toml"
    ) -> anyhow::Result<()>;
}

// build_scp_args — note capital -P for port
pub fn build_scp_args(worker: &WorkerConfig, timeout_secs: u64, extra_args: &[&str]) -> Vec<String>

// Free functions — generic over C: SshClient
pub async fn deliver_manifest<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    job_id: &JobId,
    local_manifest: &std::path::Path,
) -> anyhow::Result<String>  // returns "/tmp/smelt-<job_id>.toml"

pub async fn run_remote_job<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    remote_manifest_path: &str,
) -> anyhow::Result<i32>     // raw exit code — caller maps 0/2/other
```

## Test Plan

| Test | Type | Gate |
|------|------|------|
| `test_scp_args_build` | unit | always |
| `test_scp_args_custom_port` | unit | always |
| `test_deliver_manifest_mock` | unit (MockSshClient) | always |
| `test_run_remote_job_mock_success` | unit (MockSshClient) | always |
| `test_run_remote_job_mock_exit2` | unit (MockSshClient) | always |
| `test_manifest_delivery_and_remote_exec` | integration | `SMELT_SSH_TEST=1` |

`MockSshClient` implements all three trait methods with configurable returns. Lives in `#[cfg(test)]` block at the bottom of `ssh.rs`. Enables full unit coverage of `deliver_manifest` and `run_remote_job` without any subprocess.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust tokio subprocesses | (none needed — pattern already in codebase) | n/a |
| scp subprocess | (no specialized skill needed) | none found |

## Sources

- Codebase: `crates/smelt-cli/src/serve/ssh.rs` — S01 implementation (exec/probe patterns)
- Codebase: `crates/smelt-cli/src/serve/config.rs` — WorkerConfig with dead_code annotations to remove
- Codebase: `crates/smelt-cli/src/serve/dispatch.rs` — where S04 will wire deliver+run calls
- Codebase: `crates/smelt-cli/src/serve/types.rs` — JobId Display format
- DECISIONS.md: D111 (subprocess ssh/scp), D112 (key_env), D121 (generics not dyn), D050 (raw exit code semantics)
- S01-SUMMARY.md Forward Intelligence — scp args helper, generic callsite pattern, key_env resolution
- scp manual: `-P` uppercase for port (vs `-p` lowercase in ssh)
