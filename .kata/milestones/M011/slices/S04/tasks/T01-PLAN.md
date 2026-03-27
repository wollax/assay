---
estimated_steps: 5
estimated_files: 1
---

# T01: Write contract tests for SshSyncBackend (red state)

**Slice:** S04 — SshSyncBackend and CLI/MCP factory wiring
**Milestone:** M011

## Description

Create `crates/assay-backends/tests/ssh_backend.rs` defining the full `SshSyncBackend` contract before the implementation exists. Following the test-first discipline established by S02 (LinearBackend) and S03 (GitHubBackend), this file uses mock `scp`/`ssh` shell scripts (PATH override + `#[serial]`) to verify all 7 trait methods, `CapabilitySet::all()`, and the injection-safety guarantee for paths with spaces. The file deliberately references `assay_backends::ssh::SshSyncBackend` which does not exist yet — the expected state is a compile error for the `ssh_backend` test binary until T02.

## Steps

1. Create `crates/assay-backends/tests/ssh_backend.rs` with `#![cfg(feature = "ssh")]` at the top. Add imports: `use std::fs`, `use std::os::unix::fs::PermissionsExt`, `use serial_test::serial`, `use tempfile::TempDir`, `use assay_core::{CapabilitySet, StateBackend}`, `use assay_types::{FailurePolicy, OrchestratorPhase, OrchestratorStatus}`, `use assay_backends::ssh::SshSyncBackend`.

2. Write helper functions following the exact pattern from `github_backend.rs`:
   - `fn sample_status() -> OrchestratorStatus` — returns a minimal `OrchestratorStatus` with `run_id = "test-run-ssh-001"`
   - `fn write_mock_scp(dir: &Path, on_push: &str, on_pull: &str)` — writes `dir/scp` as a chmod 755 shell script. The script distinguishes push from pull by checking whether the LAST argument starts with a remote spec (contains `:`): if the second-to-last arg is local and last is `remote:path`, it's a push; if first non-flag arg is `remote:path` and last is a local path, it's a pull. Use `$#` and argument inspection. The `on_push` and `on_pull` params are shell fragments to execute for each case.
   - `fn write_mock_ssh(dir: &Path, cmd_handlers: &[(&str, &str)])` — writes `dir/ssh` as a chmod 755 shell script. Takes the last argument as the remote command, dispatches on prefix matches (e.g. `"mkdir"`, `"ls"`, `"rm"`). Unhandled commands exit 127.
   - `fn with_mock_path<R, F: FnOnce() -> R>(dir: &Path, f: F) -> R` — same as `with_mock_gh_path` in github_backend.rs: prepend `dir` to `PATH`, call `f`, restore original PATH. Safety comment: guarded by `#[serial]`.

3. Write 4 basic capability and push/pull tests, each annotated `#[serial]`:
   - `test_capabilities_returns_all` — constructs `SshSyncBackend::new("localhost".to_string(), "/remote/assay".to_string(), None, None, tmp.path().to_path_buf())`, asserts `backend.capabilities() == CapabilitySet::all()`; no subprocess needed.
   - `test_push_session_event_scp_args` — write mock scp that on push: writes a marker file `push_called` to a temp dir (`PUSH_MARKER` env var) and exits 0; on pull exits 1. Write mock ssh that on `mkdir` exits 0. Call `push_session_event(run_dir, &sample_status())`, assert `Ok(())`, assert `push_called` file exists.
   - `test_read_run_state_returns_deserialized_status` — write mock scp that on pull: serializes a sample status JSON to the destination path arg and exits 0. Call `read_run_state(run_dir)`, assert `Ok(Some(status))` where `status.run_id == "test-run-ssh-001"`.
   - `test_read_run_state_returns_none_when_file_missing` — write mock scp that on pull: exits 1 (simulates file not found). Call `read_run_state(run_dir)`, assert `Ok(None)`.

4. Write 4 messaging/annotation tests, each `#[serial]`:
   - `test_send_message_pushes_to_remote_inbox` — mock scp on push exits 0 and marks push called; mock ssh on `mkdir` exits 0. Call `send_message(inbox_path, "msg.json", b"hello")`, assert `Ok(())`.
   - `test_poll_inbox_pulls_and_removes_remote_files` — mock ssh on `ls /remote/assay/run/inbox` returns `"msg-001.json\nmsg-002.json"`; mock scp on pull writes fixed content to dest path; mock ssh on `rm` exits 0. Call `poll_inbox(inbox_path)`, assert `Ok(messages)` with `messages.len() == 2`.
   - `test_annotate_run_pushes_annotation_file` — mock scp on push exits 0; mock ssh on `mkdir` exits 0. Call `annotate_run(run_dir, "/some/manifest.toml")`, assert `Ok(())`.
   - `test_save_checkpoint_summary_pushes_checkpoint` — mock scp on push exits 0; mock ssh on `mkdir` exits 0. Call `save_checkpoint_summary(assay_dir, &TeamCheckpoint { ... })`, assert `Ok(())`.

5. Write 1 injection-safety test `#[serial]`:
   - `test_injection_safety_path_with_spaces` — construct `SshSyncBackend::new("localhost".to_string(), "/remote/assay dir with spaces".to_string(), None, None, tmp_path)`. Write mock scp that records its argv to a file: `echo "$@" >> $ARG_FILE`. Write mock ssh that on `mkdir` records argv and exits 0. Set `ARG_FILE` env var pointing to a temp file. Call `push_session_event(run_dir, &sample_status())`, assert `Ok(())`. Read the args file and assert: (a) the remote spec (`localhost:/remote/assay dir with spaces/...`) appears as a single token (no shell word-splitting), meaning the file contains the full path with spaces in one argument record, not split across multiple lines for the space.

## Must-Haves

- [ ] `#![cfg(feature = "ssh")]` at file top — tests only compile when `ssh` feature is enabled
- [ ] `write_mock_scp` and `write_mock_ssh` helpers create chmod 755 executable scripts
- [ ] All 9 tests annotated `#[serial]`
- [ ] All tests reference `assay_backends::ssh::SshSyncBackend` — compile intentionally fails until T02
- [ ] Injection safety test uses path with spaces in `remote_assay_dir` and verifies single-token delivery
- [ ] Each test sets up its own `TempDir` for the mock binary directory and for run/inbox dirs

## Verification

- `cargo test -p assay-backends --features ssh -- ssh_backend 2>&1 | head -5` — should show a compile error mentioning `unresolved import assay_backends::ssh` (expected red-state failure)
- The test file itself should have no logical errors (assertions are correct for the eventual implementation)

## Observability Impact

- Signals added/changed: mock scripts write marker files / arg files that tests assert against — provides direct inspection of what `Command::arg()` chains produced
- How a future agent inspects this: `cat $ARG_FILE` in the injection safety test shows the exact scp arguments; `cargo test -p assay-backends --features ssh -- --nocapture` shows any tracing output
- Failure state exposed: compile error from missing `assay_backends::ssh` module is the explicit signal that T02 is needed

## Inputs

- `crates/assay-backends/tests/github_backend.rs` — exact template for mock subprocess pattern, PATH override helper, `write_mock_gh` structure
- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait method signatures, `CapabilitySet`
- `crates/assay-types/src/state_backend.rs` — `StateBackendConfig::Ssh` shape

## Expected Output

- `crates/assay-backends/tests/ssh_backend.rs` — 9 tests defining the full SshSyncBackend contract; compile fails on `assay_backends::ssh` until T02
