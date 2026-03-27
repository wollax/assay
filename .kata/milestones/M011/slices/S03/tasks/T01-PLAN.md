---
estimated_steps: 5
estimated_files: 2
---

# T01: Contract tests for GitHubBackend (red state)

**Slice:** S03 — GitHubBackend
**Milestone:** M011

## Description

Test-first task: create 8 contract tests that define the complete `GitHubBackend` interface before implementation exists. Tests use a multi-subcommand mock `gh` shell script and PATH override with `#[serial]` for isolation. The test file will reference `crate::github::GitHubBackend` which doesn't exist yet — this is the expected red state.

## Steps

1. Add `serial_test` to `[dev-dependencies]` in `crates/assay-backends/Cargo.toml`
2. Create `crates/assay-backends/tests/github_backend.rs` gated with `#![cfg(feature = "github")]`
3. Implement test helpers:
   - `write_mock_gh(dir, handlers)` — writes a shell script at `dir/gh` that dispatches on `$1 $2` (e.g. `issue create` → echo URL, `issue comment` → echo OK, `issue view` → echo JSON). Takes a map of subcommand→behavior. Sets `0o755` permissions.
   - `with_mock_gh_path(dir, closure)` — prepends `dir` to PATH, runs closure, restores PATH. Same pattern as `crates/assay-core/tests/pr.rs`.
4. Write 8 test functions, all `#[serial]`:
   - `test_capabilities_returns_none` — construct `GitHubBackend::new(...)`, assert `capabilities() == CapabilitySet::none()`
   - `test_push_first_event_creates_issue` — mock `gh issue create` echoes `https://github.com/owner/repo/issues/42\n`; call `push_session_event`; verify `.github-issue-number` contains `42`
   - `test_push_subsequent_event_creates_comment` — pre-write `.github-issue-number` with `42`; mock `gh issue comment` exits 0; call `push_session_event`; verify no error
   - `test_read_run_state_deserializes_latest_comment` — pre-write `.github-issue-number`; mock `gh issue view` returns JSON with `comments` array containing `OrchestratorStatus` JSON body; verify deserialized status matches
   - `test_read_run_state_returns_none_without_issue_file` — no `.github-issue-number`; verify `read_run_state` returns `Ok(None)`
   - `test_send_message_returns_error` — verify `send_message` returns `Err` with `Unsupported` kind
   - `test_gh_not_found_returns_error` — set PATH to empty dir (no `gh` binary); call `push_session_event`; verify error
   - `test_gh_nonzero_exit_returns_error` — mock `gh issue create` exits 1 with stderr; verify error includes stderr content
5. Verify the test file compiles with `cargo test -p assay-backends --features github --no-run` (will fail to compile since `crate::github` doesn't exist — expected red state)

## Must-Haves

- [ ] `serial_test` added to assay-backends dev-deps
- [ ] 8 test functions covering: capabilities, first push, subsequent push, read_run_state (success + none), send_message error, gh-not-found, gh-nonzero-exit
- [ ] Mock `gh` dispatcher script handles `issue create`, `issue comment`, and `issue view` subcommands
- [ ] All tests use `#[serial]` for PATH isolation
- [ ] `--body-file -` stdin pipe pattern tested (not `--body` CLI arg) to avoid ARG_MAX issues

## Verification

- `cargo test -p assay-backends --features github --no-run 2>&1` — should fail to compile with "unresolved import `crate::github`" (expected red state, confirming tests reference the right module path)
- If the module were to exist as a stub, all 8 tests should be structurally valid shell script + assertion combinations

## Observability Impact

- Signals added/changed: None (tests only)
- How a future agent inspects this: `cargo test -p assay-backends --features github` — test names and failure messages are self-documenting
- Failure state exposed: Each test's assertion messages identify the specific contract violation

## Inputs

- `crates/assay-backends/src/linear.rs` — LinearBackend contract test patterns to follow (mockito-based, adapted to subprocess mock)
- `crates/assay-core/tests/pr.rs` — `write_fake_gh()` / `with_mock_gh_path()` patterns for mock `gh` binary
- S01 forward intelligence: `StateBackendConfig::GitHub { repo, label }` variant shape
- S03-RESEARCH.md: `gh issue create` output format (URL, not JSON), `gh issue view --json body,comments` shape, `--body-file -` for stdin piping

## Expected Output

- `crates/assay-backends/Cargo.toml` — `serial_test` added to dev-deps
- `crates/assay-backends/tests/github_backend.rs` — 8 contract tests in red state (compile error referencing `crate::github::GitHubBackend`)
