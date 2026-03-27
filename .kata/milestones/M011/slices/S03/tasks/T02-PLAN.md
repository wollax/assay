---
estimated_steps: 5
estimated_files: 4
---

# T02: Implement GitHubBackend and wire factory dispatch

**Slice:** S03 — GitHubBackend
**Milestone:** M011

## Description

Implement `GitHubBackend` in `crates/assay-backends/src/github.rs` to make all 8 T01 contract tests pass. Wire the module into `lib.rs` behind `cfg(feature = "github")` and update `factory.rs` to dispatch `StateBackendConfig::GitHub` to the real backend. Follow LinearBackend structure closely but substitute `std::process::Command` for `reqwest::blocking::Client`.

## Steps

1. Create `crates/assay-backends/src/github.rs` with:
   - `GhRunner` struct (holds `repo: String`):
     - `create_issue(title, body, label) -> Result<u64>` — runs `gh issue create --repo <repo> --title <title> --body-file - [--label <label>]` with body piped via stdin; parses issue number from URL in stdout (`.trim().rsplit('/').next().parse::<u64>()`)
     - `create_comment(issue_number, body) -> Result<()>` — runs `gh issue comment <number> --repo <repo> --body-file -` with body piped via stdin
     - `get_issue_json(issue_number) -> Result<Value>` — runs `gh issue view <number> --repo <repo> --json body,comments` and parses stdout as JSON
   - `GitHubBackend` struct (holds `repo: String`, `label: Option<String>`, internal `GhRunner`):
     - `pub fn new(repo: String, label: Option<String>) -> Self`
     - `read_issue_number(run_dir) -> Result<Option<u64>>` — reads `.github-issue-number`
     - `write_issue_number(run_dir, number) -> Result<()>` — writes `.github-issue-number`
   - `impl StateBackend for GitHubBackend`:
     - `capabilities()` → `CapabilitySet::none()` (all false)
     - `push_session_event` → check `.github-issue-number`: absent → `create_issue` + write number; present → `create_comment`
     - `read_run_state` → read number, `get_issue_json`, extract latest comment body (or issue body if no comments), deserialize as `OrchestratorStatus`
     - `send_message` / `poll_inbox` → `Err(AssayError::io("... not supported ...", Unsupported))`
     - `annotate_run` / `save_checkpoint_summary` → `Ok(())` (silent no-op, capabilities are false)
   - All `Command` calls use `.arg()` chaining (never shell string). `--repo <repo>` always passed explicitly. Body passed via `--body-file -` + `Stdio::piped()` + write to stdin. Stderr captured for error messages.
   - `tracing::info!` on issue creation (log issue number) and comment creation. `tracing::debug!` on command construction. `tracing::warn!` on non-zero exit.

2. Update `crates/assay-backends/src/lib.rs`:
   - Add `#[cfg(feature = "github")] pub mod github;`

3. Update `crates/assay-backends/src/factory.rs`:
   - Replace the current `StateBackendConfig::GitHub { .. }` arm with a `#[cfg(feature = "github")]` / `#[cfg(not(feature = "github"))]` dual-arm pattern (same as `Linear`)
   - `#[cfg(feature = "github")]`: extract `repo` and `label`, construct `Arc::new(GitHubBackend::new(repo.clone(), label.clone()))`
   - `#[cfg(not(feature = "github"))]`: `tracing::warn!` + `Arc::new(NoopBackend)`
   - Update `factory_github_returns_noop` test: when `github` feature is enabled, assert `CapabilitySet::none()` (GitHubBackend has all-false capabilities, same as NoopBackend — but the test name should reflect the real backend is used)

4. Run `cargo test -p assay-backends --features github` — all 8 contract tests + factory tests must pass

5. Run `just ready` — all 1501+ workspace tests pass, zero regression

## Must-Haves

- [ ] `GitHubBackend` implements all 7 `StateBackend` methods
- [ ] `GhRunner` uses `Command::arg()` chaining (no shell string interpolation) with `--repo` always explicit
- [ ] Body piped via `--body-file -` + `Stdio::piped()` (not `--body` CLI arg)
- [ ] Issue number parsed from `gh issue create` stdout URL via `.trim().rsplit('/').next().parse::<u64>()`
- [ ] `read_run_state` reads `gh issue view --json body,comments`, extracts latest comment body or falls back to issue body
- [ ] `.github-issue-number` file lifecycle mirrors `.linear-issue-id`
- [ ] Factory dispatch uses `#[cfg(feature = "github")]` dual-arm pattern
- [ ] `tracing::info!` / `tracing::warn!` / `tracing::debug!` at appropriate points
- [ ] All 8 T01 contract tests pass
- [ ] `just ready` green

## Verification

- `cargo test -p assay-backends --features github` — all contract tests pass
- `cargo test -p assay-backends` — factory tests pass (no regression)
- `cargo clippy -p assay-backends --features github` — zero warnings
- `just ready` — all 1501+ workspace tests pass

## Observability Impact

- Signals added/changed: `tracing::info!` on issue creation (issue_number field) and comment creation; `tracing::warn!` on gh errors with captured stderr; `tracing::debug!` on command arg construction
- How a future agent inspects this: `.github-issue-number` file in run_dir; `read_run_state()` returns the latest OrchestratorStatus; tracing events visible in structured log output
- Failure state exposed: `AssayError::Io` with operation label, path, and gh stderr. `ErrorKind::NotFound` when `gh` binary is missing. `ErrorKind::Other` for non-zero exit with stderr message.

## Inputs

- `crates/assay-backends/tests/github_backend.rs` — T01 contract tests (red state → must turn green)
- `crates/assay-backends/src/linear.rs` — LinearBackend structure template (issue ID tracking, method patterns, error handling)
- `crates/assay-backends/src/factory.rs` — existing factory with `Linear` dual-arm pattern to replicate for `GitHub`
- S03-RESEARCH.md: `gh issue create` URL format, `gh issue view --json` shape, `--body-file -` stdin pattern, `--repo` flag requirement

## Expected Output

- `crates/assay-backends/src/github.rs` — complete GitHubBackend + GhRunner implementation (~250-350 lines)
- `crates/assay-backends/src/lib.rs` — `pub mod github` behind feature gate
- `crates/assay-backends/src/factory.rs` — GitHub dual-arm dispatch + updated factory test
- All 8+ tests green, `just ready` passing
