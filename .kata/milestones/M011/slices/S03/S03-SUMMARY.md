---
id: S03
parent: M011
milestone: M011
provides:
  - GitHubBackend implementing all 7 StateBackend methods via gh CLI subprocess
  - GhRunner internal module wrapping gh issue create/comment/view
  - 8 contract tests with mock gh binary (PATH override + serial_test)
  - Factory dispatch for StateBackendConfig::GitHub behind cfg(feature = "github")
requires:
  - slice: S01
    provides: StateBackendConfig::GitHub { repo, label } variant; backend_from_config() stub; assay-backends crate scaffold
affects:
  - slice: S04
    provides: factory.rs GitHub dispatch arm; GitHubBackend struct + GhRunner API
key_files:
  - crates/assay-backends/src/github.rs
  - crates/assay-backends/tests/github_backend.rs
  - crates/assay-backends/src/lib.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-backends/Cargo.toml
key_decisions:
  - Body passed via --body-file - with Stdio::piped stdin to avoid shell quoting issues (not --body CLI arg)
  - Issue number stored as u64 in .github-issue-number matching gh URL format
  - read_run_state extracts latest comment body, falls back to issue body if no comments
  - write_mock_gh uses multi-subcommand dispatcher via $1 $2 inspection (single script, not per-test scripts)
  - with_mock_gh_path closure takes no arguments since tests already have the dir in scope
patterns_established:
  - GhRunner struct encapsulates all gh CLI interactions with Command::arg() chaining (no shell interpolation)
  - .github-issue-number file lifecycle mirrors .linear-issue-id pattern from LinearBackend (S02)
  - Factory dual-arm cfg(feature) / cfg(not(feature)) pattern replicated from Linear for GitHub
  - read_run_state falls back to issue body when no comments exist (defensive, handles newly-created issues)
observability_surfaces:
  - tracing::info! on issue creation (issue_number, repo) and comment creation
  - tracing::warn! on gh non-zero exit with captured stderr
  - tracing::debug! on command construction
  - .github-issue-number file in run_dir for issue tracking
drill_down_paths:
  - .kata/milestones/M011/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M011/slices/S03/tasks/T02-SUMMARY.md
duration: ~16m
verification_result: passed
completed_at: 2026-03-27
---

# S03: GitHubBackend

**`GitHubBackend` implementing all 7 `StateBackend` methods via `gh` CLI subprocess calls, with 8 mock-binary contract tests, and factory dispatch routing `GitHub` variant to real backend behind `cfg(feature = "github")`.**

## What Happened

**T01 (contract tests, red state):** Added `serial_test` workspace dev-dep and created `tests/github_backend.rs` gated with `#![cfg(feature = "github")]`. Defined 8 contract tests using a mock `gh` binary infrastructure: `write_mock_gh(dir, handlers)` creates a multi-subcommand dispatcher script inspecting `$1 $2`, and `with_mock_gh_path(dir, closure)` handles PATH override with `#[serial]` isolation. Tests cover capabilities, first-push issue creation, subsequent-push comment creation, read_run_state deserialization, None-without-issue-file, send_message error, gh-not-found error, and gh-nonzero-exit error. Confirmed red-state compile error (`unresolved import assay_backends::github`).

**T02 (implementation):** Created `crates/assay-backends/src/github.rs` with `GhRunner` (low-level CLI wrapper) and `GitHubBackend` (StateBackend implementor). `GhRunner` holds `repo: String` and provides `create_issue` (parses issue number from URL stdout), `create_comment` (body via `--body-file -` + `Stdio::piped()`), and `get_issue_json`. All commands use `Command::arg()` chaining with explicit `--repo` flag and stderr capture. `GitHubBackend` returns `CapabilitySet::none()`, stores issue number in `.github-issue-number`, and falls back to issue body when no comments exist. Updated `lib.rs` and `factory.rs` with dual-arm `cfg(feature = "github")` dispatch matching the Linear pattern.

## Verification

- `cargo test -p assay-backends --features github` — 13 tests pass (8 contract + 5 factory) ✅
- `cargo test -p assay-backends` — 5 factory tests pass, no regression ✅
- `cargo clippy -p assay-backends --features github` — zero warnings ✅
- `just ready` — all checks passed (fmt, lint, test, deny); 1501 total workspace tests ✅

## Requirements Advanced

- R077 (GitHubBackend) — advanced from active → validated by contract tests proving all 7 StateBackend methods

## Requirements Validated

- R077 — `GitHubBackend` implements all 7 `StateBackend` methods; `push_session_event` creates issue on first call and comments on subsequent; `read_run_state` deserializes latest comment (with body fallback); `capabilities()` returns all-false (`CapabilitySet::none()`); 8 mock-subprocess contract tests pass; factory dispatches `GitHub → GitHubBackend`; `just ready` green with 1501 tests

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- Factory test renamed from `factory_github_returns_noop` (from S01 stub) to `factory_github_capabilities` — both assert `CapabilitySet::none()`, but the name now accurately reflects that the real `GitHubBackend` is used when the feature is enabled.
- `read_run_state` falls back to issue body when no comments exist — not in the original plan spec but defensively correct for newly-created issues.
- `with_mock_gh_path` closure takes no arguments (unlike PR tests which pass `&Path`) — tests already have `dir` in scope so no argument needed.
- Mock `read_run_state` test writes JSON to a temp file and cats it from the shell script rather than embedding JSON directly — avoids nested quoting issues with single quotes in JSON strings.

## Known Limitations

- GitHubBackend is contract-tested with mock `gh` binary only; real `gh` CLI validation against a GitHub repo is UAT-only.
- `capabilities()` returns `CapabilitySet::none()` (no annotations, messaging, gossip, checkpoints) — GitHub Issues have no inbox/outbox semantics.
- `send_message` and `poll_inbox` return `Err(Unsupported)` — this is by design per R077.
- `annotate_run` and `save_checkpoint_summary` are silent no-ops (capability false).

## Follow-ups

- S04 wires `assay-cli` and `assay-mcp` construction sites to use `backend_from_config()` instead of hardcoded `LocalFsBackend::new(...)`.
- S04 implements `SshSyncBackend` with `CapabilitySet::all()` and scp injection-safe arg construction.
- UAT: validate `GitHubBackend` with real `gh` CLI against a test GitHub repo.

## Files Created/Modified

- `crates/assay-backends/src/github.rs` — New: `GhRunner` + `GitHubBackend` implementing all 7 `StateBackend` methods (~320 lines)
- `crates/assay-backends/tests/github_backend.rs` — New: 8 contract tests with mock gh binary infrastructure (~260 lines)
- `crates/assay-backends/src/lib.rs` — Added `pub mod github` behind `cfg(feature = "github")`
- `crates/assay-backends/src/factory.rs` — GitHub dual-arm dispatch + renamed factory test
- `crates/assay-backends/Cargo.toml` — Added `serial_test.workspace = true` to dev-deps

## Forward Intelligence

### What the next slice should know
- The `cfg(feature)` / `cfg(not(feature))` dual-arm pattern in `factory.rs` is established for both `linear` and `github`; S04's `ssh` feature should follow the same pattern exactly.
- `GhRunner` uses `Command::arg()` chaining throughout — S04's `SshSyncBackend` scp wrappers should do the same (no shell string interpolation).
- The `.github-issue-number` file lifecycle (create on first push, read on subsequent) mirrors `.linear-issue-id` — both files live in `run_dir`.

### What's fragile
- Mock gh binary tests are path-dependent (`#[serial]` + env var mutation) — parallel test execution will fail if the serial attribute is removed.
- `read_run_state` JSON parsing assumes the `gh issue view --json body,comments` output shape; any `gh` CLI version change breaking this shape will fail silently at deserialization.

### Authoritative diagnostics
- `.github-issue-number` file in `run_dir` — trust this file to know which GitHub issue a run is mapped to.
- `tracing::warn!` with captured stderr — when `gh` exits nonzero, this is the primary failure signal.

### What assumptions changed
- Original plan called `annotate_run` returning `Ok(())` silent no-op — this is what shipped. No capability false → error escalation needed.
