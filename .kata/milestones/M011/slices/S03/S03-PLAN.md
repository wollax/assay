# S03: GitHubBackend

**Goal:** `GitHubBackend` implements all 7 `StateBackend` methods via `gh` CLI subprocess calls; contract tests with a mock `gh` binary pass; `backend_from_config()` dispatches `GitHub` variant to `GitHubBackend`; `just ready` green.

**Demo:** `cargo test -p assay-backends --features github` runs contract tests proving `push_session_event` creates/comments on GitHub issues, `read_run_state` reads the latest comment, capabilities are all-false, and unsupported methods return appropriate errors. Factory dispatch routes `GitHub` → `GitHubBackend`.

## Must-Haves

- `GitHubBackend` struct implementing all 7 `StateBackend` methods behind `cfg(feature = "github")`
- `GhRunner` internal module wrapping `std::process::Command` calls for `gh issue create`, `gh issue comment`, `gh issue view --json`
- `push_session_event`: first call → `gh issue create` (parse issue number from stdout URL), subsequent calls → `gh issue comment --body-file -` (stdin pipe to avoid ARG_MAX)
- `read_run_state`: `gh issue view <number> --repo <repo> --json body,comments` → deserialize latest comment body as `OrchestratorStatus`
- `.github-issue-number` file-per-run-dir tracking (parallel to LinearBackend's `.linear-issue-id`)
- `capabilities()` returns all-false (`CapabilitySet::none()` — no annotations, no messaging, no gossip, no checkpoints)
- `send_message`, `poll_inbox` return `Err` (unsupported); `annotate_run`, `save_checkpoint_summary` return `Ok(())` (silent no-op, capability false)
- Contract tests with mock `gh` binary using PATH override + `#[serial]`
- `backend_from_config()` updated: `GitHub` variant → `GitHubBackend::new(...)` with `#[cfg(feature = "github")]` dual-arm pattern
- `serial_test` added to `assay-backends` dev-deps
- `just ready` green with all existing tests passing

## Proof Level

- This slice proves: contract (mock subprocess tests verify arg shapes and data flow)
- Real runtime required: no (mock `gh` binary substitutes for real GitHub CLI)
- Human/UAT required: yes (real `gh` CLI against a test repo for production validation)

## Verification

- `cargo test -p assay-backends --features github` — all contract tests pass (capabilities, first push creates issue, subsequent push creates comment, read_run_state deserializes, read_run_state returns None without issue file, unsupported methods error, gh-not-found error handling)
- `cargo test -p assay-backends` — factory dispatch tests pass (GitHub variant → correct capabilities)
- `cargo clippy -p assay-backends --features github` — zero warnings
- `just ready` — all 1501+ workspace tests pass, zero regression

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on issue creation (logs issue number) and comment creation; `tracing::warn!` on `gh` errors (stderr captured); `tracing::debug!` on command construction
- Inspection surfaces: `.github-issue-number` file in `run_dir` maps each run to a GitHub issue; `read_run_state` returns latest status as `OrchestratorStatus`
- Failure visibility: `AssayError::Io` with operation labels for all failure modes; `gh` stderr captured and included in error messages; `ErrorKind::NotFound` distinguished from auth/network errors
- Redaction constraints: none (no secrets — `gh` uses its own auth token store)

## Integration Closure

- Upstream surfaces consumed: `StateBackend` trait + `CapabilitySet` from `assay-core::state_backend`; `StateBackendConfig::GitHub { repo, label }` from `assay-types`; `OrchestratorStatus` + `TeamCheckpoint` from `assay-types`; `NoopBackend` from `assay-core`
- New wiring introduced in this slice: `backend_from_config()` GitHub arm dispatches to real `GitHubBackend` instead of `NoopBackend`; `pub mod github` in `lib.rs` behind `cfg(feature = "github")`
- What remains before the milestone is truly usable end-to-end: S04 (SshSyncBackend + CLI/MCP construction site wiring)

## Tasks

- [ ] **T01: Contract tests for GitHubBackend (red state)** `est:25m`
  - Why: Test-first — define the contract before implementation. Tests will fail to compile (red state) until T02 implements `GitHubBackend`.
  - Files: `crates/assay-backends/tests/github_backend.rs`, `crates/assay-backends/Cargo.toml`
  - Do: Add `serial_test` to dev-deps. Create test file behind `cfg(feature = "github")` with 8 contract tests using mock `gh` binary (multi-subcommand dispatcher script). Tests: capabilities returns none, first push creates issue (verify args + parse issue number from URL), subsequent push comments (verify `--body-file -` stdin pipe), read_run_state deserializes latest comment, read_run_state returns None without issue file, send_message returns Err, gh-not-found returns Err, gh-auth-error returns Err. Each test uses `write_mock_gh()` helper (dispatching script that inspects `$1 $2`) and `with_mock_gh_path()` helper with `#[serial]`.
  - Verify: `cargo test -p assay-backends --features github` — tests compile but fail (GitHubBackend not defined yet)
  - Done when: All 8 test functions exist, reference `crate::github::GitHubBackend`, and compile to red state

- [ ] **T02: Implement GitHubBackend and wire factory dispatch** `est:30m`
  - Why: Make all T01 tests pass by implementing the real `GitHubBackend` struct and updating factory dispatch.
  - Files: `crates/assay-backends/src/github.rs`, `crates/assay-backends/src/lib.rs`, `crates/assay-backends/src/factory.rs`
  - Do: Create `github.rs` with `GhRunner` (wraps `Command` calls: `create_issue` parses URL for issue number, `create_comment` uses `--body-file -` with stdin pipe, `get_issue_json` returns parsed JSON). `GitHubBackend` struct holds `repo: String`, `label: Option<String>`. Implements all 7 `StateBackend` methods. `.github-issue-number` file tracking. Update `lib.rs` with `#[cfg(feature = "github")] pub mod github`. Update `factory.rs` with `#[cfg(feature = "github")]` / `#[cfg(not(feature = "github"))]` dual-arm pattern. Update factory test for GitHub to check real capabilities when feature enabled.
  - Verify: `cargo test -p assay-backends --features github` — all 8 contract tests pass; `cargo test -p assay-backends` — factory tests pass; `just ready` green
  - Done when: All contract tests green, factory dispatch routes GitHub → GitHubBackend, `just ready` passes with 1501+ tests

## Files Likely Touched

- `crates/assay-backends/Cargo.toml` — add `serial_test` dev-dep
- `crates/assay-backends/tests/github_backend.rs` — contract tests (new)
- `crates/assay-backends/src/github.rs` — GitHubBackend implementation (new)
- `crates/assay-backends/src/lib.rs` — add `pub mod github` behind feature gate
- `crates/assay-backends/src/factory.rs` — update GitHub dispatch arm + factory test
