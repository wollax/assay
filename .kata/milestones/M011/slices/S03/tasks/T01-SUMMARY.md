---
id: T01
parent: S03
milestone: M011
provides:
  - 8 contract tests defining GitHubBackend interface
  - Mock gh binary infrastructure (write_mock_gh + with_mock_gh_path)
  - serial_test dev-dependency for assay-backends
key_files:
  - crates/assay-backends/tests/github_backend.rs
  - crates/assay-backends/Cargo.toml
key_decisions:
  - Mock gh uses multi-subcommand dispatcher via $1 $2 inspection (not separate scripts per test)
  - read_run_state test uses temp file for JSON output (avoids shell quoting issues with nested JSON)
patterns_established:
  - write_mock_gh(dir, handlers) with subcommand dispatch for multi-operation mock gh tests
  - with_mock_gh_path(dir, closure) for PATH override with #[serial] isolation
observability_surfaces:
  - none (tests only)
duration: 8m
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T01: Contract tests for GitHubBackend (red state)

**Created 8 contract tests defining the complete GitHubBackend interface with mock `gh` binary infrastructure; confirmed expected red-state compile error.**

## What Happened

Added `serial_test` to `assay-backends` dev-deps and created `github_backend.rs` test file gated with `#![cfg(feature = "github")]`. The file defines 8 test functions covering the full `StateBackend` contract for GitHubBackend:

1. `test_capabilities_returns_none` — verifies all-false capabilities
2. `test_push_first_event_creates_issue` — mock `gh issue create` echoes URL, verifies `.github-issue-number` contains parsed `42`
3. `test_push_subsequent_event_creates_comment` — pre-writes issue file, mock `gh issue comment` with stdin pipe (`--body-file -`)
4. `test_read_run_state_deserializes_latest_comment` — mock `gh issue view` returns JSON with comments array, verifies OrchestratorStatus deserialization
5. `test_read_run_state_returns_none_without_issue_file` — no issue file, returns `Ok(None)`
6. `test_send_message_returns_error` — verifies Unsupported error
7. `test_gh_not_found_returns_error` — empty PATH, verifies error on missing `gh`
8. `test_gh_nonzero_exit_returns_error` — mock exits 1 with stderr, verifies error includes stderr content

Helper functions: `write_mock_gh(dir, handlers)` creates a dispatcher script inspecting `$1 $2`, and `with_mock_gh_path(dir, closure)` handles PATH override/restore.

## Verification

- `cargo test -p assay-backends --features github --no-run 2>&1` → **expected compile error**: `unresolved import assay_backends::github` — confirms tests reference the correct module path ✅
- `cargo test -p assay-backends` (without github feature) → **0 tests, ok** — cfg gate works correctly, no regression ✅

### Slice-level checks (partial — T01 is intermediate):
- `cargo test -p assay-backends --features github` → compile error (expected, T02 will fix) ✅
- `cargo test -p assay-backends` → passes (no regression) ✅
- `cargo clippy -p assay-backends --features github` → N/A (can't lint until code compiles)
- `just ready` → not yet (blocked on T02 implementation)

## Diagnostics

Test names and assertion messages are self-documenting. Run `cargo test -p assay-backends --features github` after T02 to verify all 8 contract tests pass.

## Deviations

- `read_run_state` test writes mock JSON to a temp file and cats it from the shell script, rather than embedding JSON directly in the shell script — avoids nested quoting issues with single quotes in JSON strings.
- `with_mock_gh_path` closure takes no arguments (unlike pr.rs which passes `&Path`) since tests already have the dir in scope.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-backends/Cargo.toml` — added `serial_test.workspace = true` to dev-deps
- `crates/assay-backends/tests/github_backend.rs` — 8 contract tests for GitHubBackend (new file, ~260 lines)
