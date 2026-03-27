---
estimated_steps: 7
estimated_files: 4
---

# T02: Implement SshSyncBackend and wire into factory

**Slice:** S04 — SshSyncBackend and CLI/MCP factory wiring
**Milestone:** M011

## Description

Implement `crates/assay-backends/src/ssh.rs` with `ScpRunner` (low-level scp/ssh command wrapper using `Command::arg()` chaining, D163) and `SshSyncBackend` (all 7 `StateBackend` methods). Register the module in `lib.rs`. Replace the `NoopBackend` stub in `factory.rs` with the real `SshSyncBackend` dispatch behind `#[cfg(feature = "ssh")]`. All 9 T01 contract tests should pass after this task.

## Steps

1. Create `crates/assay-backends/src/ssh.rs`. Add module-level doc comment. Define `ScpRunner`:
   ```rust
   struct ScpRunner {
       host: String,
       user: Option<String>,
       port: Option<u16>,
   }
   ```
   Implement `ScpRunner`:
   - `fn host_spec(&self) -> String` — returns `"user@host"` or `"host"` based on `user` field
   - `fn remote_spec(&self, remote_path: &str) -> String` — returns `"host_spec:remote_path"` as a single string
   - `fn build_scp_base(&self) -> Command` — creates `Command::new("scp")`; if port is set, chains `.arg("-P").arg(port_str)` (uppercase P — scp-specific)
   - `fn scp_push(&self, local: &Path, remote_path: &str) -> assay_core::Result<()>` — calls `build_scp_base()`, chains `.arg(local).arg(self.remote_spec(remote_path))`, spawns, waits; on non-zero exit, captures stderr and returns `Err(AssayError::io("scp push", local, io_err_from_stderr))`
   - `fn scp_pull(&self, remote_path: &str, local: &Path) -> assay_core::Result<()>` — same pattern, args are `.arg(self.remote_spec(remote_path)).arg(local)`
   - `fn ssh_run(&self, remote_cmd: &str) -> assay_core::Result<String>` — builds `Command::new("ssh")`; if port set, `.arg("-p").arg(port_str)` (lowercase p — ssh-specific); `.arg(self.host_spec()).arg(remote_cmd)`; captures stdout; on non-zero exit returns error with stderr content
   - Note: `remote_cmd` for `ssh_run` is a simple string like `"mkdir -p /path"` or `"ls /path"`. Since this goes through the remote shell, paths with spaces need shell-quoting inside the command string. Helper: `fn shell_quote(s: &str) -> String` that wraps the value in single quotes, escaping any embedded single quotes using `"'\\''"`  pattern.

2. Define `SshSyncBackend`:
   ```rust
   pub struct SshSyncBackend {
       runner: ScpRunner,
       remote_assay_dir: String,
       local_assay_dir: PathBuf,
   }
   ```
   Implement constructor:
   ```rust
   pub fn new(host: String, remote_assay_dir: String, user: Option<String>, port: Option<u16>, local_assay_dir: PathBuf) -> Self
   ```
   Implement helper `fn to_remote_path(&self, local: &Path) -> String` — strips the `local_assay_dir` prefix from `local` using `local.strip_prefix(&self.local_assay_dir)`, joins the remainder with `remote_assay_dir`. If stripping fails (local path outside assay_dir), fall back to using the full local path's file_name joined to remote_assay_dir.

3. Implement `StateBackend` for `SshSyncBackend`:
   - `capabilities()` → `CapabilitySet::all()`
   - `push_session_event(run_dir, status)`:
     1. Serialize `status` to JSON bytes
     2. Write to a `NamedTempFile` in `local_assay_dir` (or system temp) — use `std::env::temp_dir()`
     3. `ssh_run(&format!("mkdir -p {}", shell_quote(&self.to_remote_path(run_dir))))`
     4. `scp_push(tmp_path, &self.to_remote_path(run_dir) + "/state.json")`
   - `read_run_state(run_dir)`:
     1. Create a temp file destination
     2. `scp_pull(remote_state_json, tmp_path)` — if scp returns an error, return `Ok(None)` (file doesn't exist yet)
     3. Deserialize from temp file bytes, return `Ok(Some(status))`
   - `send_message(inbox_path, name, contents)`:
     1. Write contents to temp file
     2. `ssh_run(mkdir -p remote_inbox_path)`
     3. `scp_push(tmp, remote_inbox/name)`
   - `poll_inbox(inbox_path)`:
     1. `ssh_run(ls remote_inbox_path)` — if error (NotFound), return `Ok(vec![])`
     2. Parse filenames from output (split on newlines, filter empty)
     3. For each filename: `scp_pull(remote_inbox/filename, local_tmp)`, read bytes
     4. For each pulled file: `ssh_run(rm remote_inbox/filename)` — warn on failure, don't fail
     5. Return collected `Vec<(String, Vec<u8>)>`
   - `annotate_run(run_dir, manifest_path)`:
     1. Write manifest_path bytes to temp file
     2. `ssh_run(mkdir -p remote_run_dir)`
     3. `scp_push(tmp, remote_run_dir/annotation.txt)`
   - `save_checkpoint_summary(assay_dir, checkpoint)`:
     1. Serialize checkpoint to JSON
     2. Write to temp file
     3. Remote dir is `remote_assay_dir/checkpoints`
     4. `ssh_run(mkdir -p remote_checkpoints_dir)`
     5. `scp_push(tmp, remote_checkpoints/checkpoint_id.json)` — use `checkpoint.run_id` as filename if available, else a UUID via `uuid::Uuid::new_v4()` … actually use a timestamp-based name to avoid adding uuid dep: `format!("{}.json", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())`

4. In `crates/assay-backends/src/lib.rs`, add:
   ```rust
   #[cfg(feature = "ssh")]
   pub mod ssh;
   ```

5. In `crates/assay-backends/src/factory.rs`, replace the current unconditional `StateBackendConfig::Ssh { .. }` arm with:
   ```rust
   #[cfg(feature = "ssh")]
   StateBackendConfig::Ssh { host, remote_assay_dir, user, port } => {
       Arc::new(crate::ssh::SshSyncBackend::new(
           host.clone(),
           remote_assay_dir.clone(),
           user.clone(),
           *port,
           assay_dir,
       ))
   }
   #[cfg(not(feature = "ssh"))]
   StateBackendConfig::Ssh { .. } => {
       tracing::warn!(
           backend = "ssh",
           "SshSyncBackend requires the `ssh` feature — falling back to NoopBackend; \
            all state writes will be discarded"
       );
       Arc::new(NoopBackend)
   }
   ```

6. Run `cargo test -p assay-backends --features ssh -- ssh_backend` and fix any compilation errors in `ssh.rs` until all 9 tests pass.

7. Run `cargo test -p assay-backends` (without ssh feature) to confirm the non-feature build still compiles and passes factory dispatch tests.

## Must-Haves

- [ ] `Command::arg()` chaining used throughout — no `sh -c` with user-supplied path values (D163)
- [ ] scp port flag is uppercase `-P`; ssh port flag is lowercase `-p` — correctly differentiated
- [ ] `CapabilitySet::all()` returned from `capabilities()`
- [ ] `read_run_state` returns `Ok(None)` on scp pull failure (file not found), not `Err`
- [ ] `poll_inbox` degrades gracefully when remote inbox dir doesn't exist (ssh ls fails → `Ok(vec![])`)
- [ ] `shell_quote` helper used for all paths passed as arguments to remote ssh commands
- [ ] `#[cfg(feature = "ssh")]` on `pub mod ssh` in `lib.rs`
- [ ] Both cfg arms present in `factory.rs` (`feature = "ssh"` and `not(feature = "ssh")`)
- [ ] All 9 T01 contract tests pass with `--features ssh`

## Verification

- `cargo test -p assay-backends --features ssh -- ssh_backend` — all 9 tests pass
- `cargo test -p assay-backends` — factory dispatch tests pass (no ssh feature), no compile errors
- `grep -n "sh -c\|shell" crates/assay-backends/src/ssh.rs` — no shell string interpolation for user-provided path values (doc comment on `ssh_run` noting that remote_cmd passes through remote shell is acceptable)
- `cargo clippy -p assay-backends --features ssh -- -D warnings` — no warnings

## Observability Impact

- Signals added/changed: `tracing::debug!` at the start of `scp_push`, `scp_pull`, `ssh_run` logging the command name and (non-sensitive) arg count — not logging file content or credentials
- How a future agent inspects this: `cargo test -p assay-backends --features ssh -- --nocapture` shows debug traces; injection safety test's ARG_FILE shows actual argv tokens
- Failure state exposed: `AssayError::io("scp push", path, e)` carries operation name + path for all scp failures; captured stderr from failed subprocesses included in error

## Inputs

- `crates/assay-backends/tests/ssh_backend.rs` (T01) — the contract that implementation must satisfy
- `crates/assay-backends/src/github.rs` — template for Command::arg() pattern, cache file pattern, error wrapping
- `crates/assay-core/src/state_backend.rs` — StateBackend trait signatures, AssayError, CapabilitySet
- `crates/assay-backends/src/factory.rs` — existing Ssh stub arm to replace

## Expected Output

- `crates/assay-backends/src/ssh.rs` — complete SshSyncBackend implementation (~200 lines)
- `crates/assay-backends/src/lib.rs` — `pub mod ssh` added behind `cfg(feature = "ssh")`
- `crates/assay-backends/src/factory.rs` — Ssh arm dispatches to SshSyncBackend when feature enabled
- All 9 ssh_backend contract tests pass with `--features ssh`
