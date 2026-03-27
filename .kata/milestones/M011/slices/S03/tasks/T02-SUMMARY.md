---
id: T02
parent: S03
milestone: M011
provides:
  - GitHubBackend implementing all 7 StateBackend methods via gh CLI
  - GhRunner low-level gh CLI wrapper (create_issue, create_comment, get_issue_json)
  - Factory dispatch for StateBackendConfig::GitHub behind cfg(feature = "github")
key_files:
  - crates/assay-backends/src/github.rs
  - crates/assay-backends/src/lib.rs
  - crates/assay-backends/src/factory.rs
key_decisions:
  - Body passed via --body-file - with Stdio::piped stdin (not --body CLI arg) to avoid shell quoting issues
  - Issue number stored as u64 (not string) in .github-issue-number, matching gh URL format
  - read_run_state extracts latest comment body (last in array), falls back to issue body if no comments
patterns_established:
  - GhRunner struct encapsulates all gh CLI interactions with Command::arg() chaining
  - .github-issue-number file lifecycle mirrors .linear-issue-id pattern from LinearBackend
  - Factory dual-arm cfg(feature) / cfg(not(feature)) pattern replicated from Linear for GitHub
observability_surfaces:
  - tracing::info! on issue creation (issue_number, repo) and comment creation
  - tracing::warn! on gh non-zero exit with captured stderr
  - tracing::debug! on command construction
  - .github-issue-number file in run_dir for issue tracking
duration: ~8 minutes
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T02: Implement GitHubBackend and wire factory dispatch

**Implemented GitHubBackend with GhRunner wrapper executing gh CLI subprocess calls for issue create/comment/view, wired factory dispatch behind `cfg(feature = "github")`.**

## What Happened

Created `crates/assay-backends/src/github.rs` with two structs:

- `GhRunner`: Low-level wrapper holding `repo: String` with three methods — `create_issue` (parses issue number from URL stdout), `create_comment`, and `get_issue_json`. All use `Command::arg()` chaining with `--repo` explicit, body via `--body-file -` + `Stdio::piped()`, and stderr captured for error reporting.

- `GitHubBackend`: Implements `StateBackend` with `CapabilitySet::none()`. `push_session_event` creates an issue on first call (caching number in `.github-issue-number`) or appends a comment on subsequent calls. `read_run_state` fetches `gh issue view --json body,comments` and deserializes the latest comment body (or issue body fallback). `send_message`/`poll_inbox` return Unsupported errors. `annotate_run`/`save_checkpoint_summary` are silent no-ops.

Updated `lib.rs` to expose `pub mod github` behind `cfg(feature = "github")`. Updated `factory.rs` with dual-arm `cfg` dispatch matching the Linear pattern, and renamed the factory test from `factory_github_returns_noop` to `factory_github_capabilities` to reflect the real backend is now used when the feature is enabled.

## Verification

- `cargo test -p assay-backends --features github` — all 13 tests pass (8 contract + 5 factory)
- `cargo clippy -p assay-backends --features github` — zero warnings
- `just ready` — all checks passed (fmt, lint, test, deny)

## Diagnostics

- `.github-issue-number` file in `run_dir` maps each run to a GitHub issue number
- `read_run_state()` returns the latest OrchestratorStatus from the issue's most recent comment
- `tracing::info!` events on issue creation (with issue_number field) and comment creation
- `tracing::warn!` on gh errors with captured stderr content
- `AssayError::Io` with operation labels for all failure modes; `ErrorKind::NotFound` when `gh` binary is missing

## Deviations

- Renamed factory test from `factory_github_returns_noop` to `factory_github_capabilities` — the test still asserts `CapabilitySet::none()` which is correct for both GitHubBackend and NoopBackend, but the name now accurately reflects the real backend is used when the feature is enabled.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-backends/src/github.rs` — New file: GitHubBackend + GhRunner implementation (~320 lines)
- `crates/assay-backends/src/lib.rs` — Added `pub mod github` behind feature gate
- `crates/assay-backends/src/factory.rs` — GitHub dual-arm dispatch + renamed factory test
