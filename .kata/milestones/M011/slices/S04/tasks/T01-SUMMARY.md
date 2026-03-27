---
id: T01
parent: S04
milestone: M011
provides:
  - 9 contract tests defining the full SshSyncBackend StateBackend API surface
  - Mock scp/ssh helper functions for subprocess testing pattern
  - Injection safety test for paths with spaces
key_files:
  - crates/assay-backends/tests/ssh_backend.rs
key_decisions:
  - Followed github_backend.rs mock subprocess pattern (write_mock_scp/ssh + PATH override + #[serial])
  - Mock scp distinguishes push/pull by inspecting last two positional args for remote spec (:)
  - Mock ssh dispatches on shell case-match prefix of last argument
patterns_established:
  - write_mock_scp(dir, on_push, on_pull) — push/pull direction detection via arg inspection
  - write_mock_ssh(dir, cmd_handlers) — prefix-match dispatch on remote command
  - with_mock_path(dir, f) — PATH override helper (same as with_mock_gh_path)
observability_surfaces:
  - Mock scripts write marker files and arg logs for direct test inspection
  - scp_args.log in injection safety test shows exact argv delivered to scp
duration: 5m
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T01: Write contract tests for SshSyncBackend (red state)

**Created `ssh_backend.rs` with 9 `#[serial]` contract tests defining the full SshSyncBackend interface before implementation exists.**

## What Happened

Created `crates/assay-backends/tests/ssh_backend.rs` following the exact test-first pattern from `github_backend.rs`. The file:

1. References `assay_backends::ssh::SshSyncBackend` which doesn't exist yet — intentional compile error until T02.
2. Defines 3 helper functions: `write_mock_scp` (direction-aware mock via arg inspection), `write_mock_ssh` (prefix-match dispatch), `with_mock_path` (PATH override).
3. Contains 9 tests covering all 7 `StateBackend` methods, `CapabilitySet::all()`, and injection safety:
   - `test_capabilities_returns_all` — asserts `CapabilitySet::all()`
   - `test_push_session_event_scp_args` — verifies scp push via marker file
   - `test_read_run_state_returns_deserialized_status` — verifies scp pull + JSON deserialization
   - `test_read_run_state_returns_none_when_file_missing` — verifies graceful None on scp failure
   - `test_send_message_pushes_to_remote_inbox` — verifies message push
   - `test_poll_inbox_pulls_and_removes_remote_files` — verifies ls + pull + rm sequence
   - `test_annotate_run_pushes_annotation_file` — verifies annotation push
   - `test_save_checkpoint_summary_pushes_checkpoint` — verifies checkpoint push with real `TeamCheckpoint`
   - `test_injection_safety_path_with_spaces` — verifies `Command::arg()` preserves spaces as single token

## Verification

- `cargo test -p assay-backends --features ssh -- ssh_backend 2>&1 | head -10` → compile error `unresolved import assay_backends::ssh` (expected red state)
- File has `#![cfg(feature = "ssh")]` at top — only compiles when ssh feature enabled
- All 9 tests annotated `#[serial]`
- `grep -c "#\[test\]"` → 9; `grep -c "#\[serial\]"` → 9 (on test fns)

### Slice-level verification (partial — T01 is intermediate):
- `cargo test -p assay-backends --features ssh` — expected compile error (SshSyncBackend not yet implemented) ✓
- `cargo test -p assay-cli --features orchestrate` — not yet relevant (T03)
- `cargo test -p assay-mcp` — not yet relevant (T03)
- `just ready` — not yet relevant (T02/T03 needed first)

## Diagnostics

- Mock scp/ssh scripts write marker files (`push_called`) and arg logs (`scp_args.log`) to temp dirs
- Injection safety test's `scp_args.log` can be inspected with `--nocapture` to see exact argv
- `cargo test -p assay-backends --features ssh -- --nocapture` will show tracing output once T02 ships

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-backends/tests/ssh_backend.rs` — 9 contract tests for SshSyncBackend (red state)
