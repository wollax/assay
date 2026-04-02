---
estimated_steps: 5
estimated_files: 5
---

# T01: GhClient trait, SubprocessGhClient, and MockGhClient

**Slice:** S03 — GitHub Issues Tracker Backend
**Milestone:** M012

## Description

Create the `serve/github/` module hierarchy with the `GhClient` trait (async, RPITIT per D019), `SubprocessGhClient` (shells out to `gh` CLI via `tokio::process::Command`, discovers binary via `which::which`), and `MockGhClient` (VecDeque-based test double matching MockSshClient). This is the abstraction layer that isolates `gh` CLI subprocess concerns from TrackerSource protocol, making both independently testable.

## Steps

1. Create `crates/smelt-cli/src/serve/github/mod.rs` with:
   - `GhIssue` struct (number: u64, title: String, body: String, url: String) with Deserialize for `gh --json` output
   - `GhClient` trait with RPITIT async methods: `list_issues(repo, label, limit) -> Result<Vec<GhIssue>>`, `edit_labels(repo, number, add_labels, remove_labels) -> Result<()>`, `create_label(repo, name) -> Result<()>`, `auth_status() -> Result<()>`
   - Re-exports of `SubprocessGhClient` and public types
   - Register `pub mod github` in `serve/mod.rs`

2. Create `crates/smelt-cli/src/serve/github/client.rs` with `SubprocessGhClient`:
   - `gh_binary()` using `which::which("gh")` returning `SmeltError::Tracker` on missing
   - `list_issues`: runs `gh issue list -R {repo} --label {label} --json number,title,body,url --limit {limit}`, parses JSON array via serde_json
   - `edit_labels`: runs `gh issue edit -R {repo} {number} --add-label {add} --remove-label {remove}` (combine in single call)
   - `create_label`: runs `gh label create -R {repo} "{name}" --force`
   - `auth_status`: runs `gh auth status`, returns error on non-zero exit
   - All methods use `tracing::debug!` for entry, `tracing::warn!` on non-zero exit
   - All methods return `SmeltError::Tracker` with `operation` field matching the method name

3. Create `crates/smelt-cli/src/serve/github/mock.rs` with `MockGhClient`:
   - `Arc<Mutex<VecDeque<Result>>>` fields for each method (matching MockSshClient exactly)
   - Builder methods: `with_list_result()`, `with_edit_result()`, `with_create_label_result()`, `with_auth_result()`
   - Implement `GhClient` trait popping from queues

4. Add unit tests in `mock.rs` (or a tests section in `mod.rs`):
   - `test_mock_list_issues_returns_queued_results`
   - `test_mock_edit_labels_returns_queued_results`
   - `test_mock_create_label_returns_queued_results`
   - `test_mock_auth_status_returns_queued_results`
   - `test_mock_exhausted_queue_returns_error`
   - `test_gh_issue_deserialize_from_json` (serde round-trip for GhIssue)
   - `test_subprocess_gh_client_binary_discovery` (which::which for `gh` — may skip if gh not installed)
   - `test_gh_client_trait_compiles` (compile-test with DummyGhClient)

5. Verify: `cargo test -p smelt-cli --lib -- serve::github`, `cargo clippy --workspace -- -D warnings`, `cargo doc --workspace --no-deps`

## Must-Haves

- [ ] `GhClient` trait defined with 4 async methods using RPITIT (no `#[async_trait]`)
- [ ] `SubprocessGhClient` discovers `gh` via `which::which` and shells out via `tokio::process::Command`
- [ ] `MockGhClient` follows VecDeque pattern from `MockSshClient` for all 4 methods
- [ ] `GhIssue` struct deserializes from `gh --json` output format
- [ ] ≥8 unit tests pass covering mock, serde, and trait compilation
- [ ] `cargo clippy` and `cargo doc` clean

## Verification

- `cargo test -p smelt-cli --lib -- serve::github` — ≥8 tests pass
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Observability Impact

- Signals added/changed: `tracing::debug!` on every `gh` subprocess invocation with full command + args; `tracing::warn!` on non-zero exit codes with stderr
- How a future agent inspects this: grep for `SmeltError::Tracker { operation: "gh_binary" }` or `operation: "auth_status"` in error chains
- Failure state exposed: Missing `gh` binary produces structured `SmeltError::Tracker`; auth failure includes stderr from `gh auth status`

## Inputs

- `crates/smelt-cli/src/serve/ssh/mod.rs` — SshClient trait pattern to mirror
- `crates/smelt-cli/src/serve/ssh/client.rs` — SubprocessSshClient pattern for binary discovery and Command execution
- `crates/smelt-cli/src/serve/ssh/mock.rs` — MockSshClient VecDeque pattern
- `crates/smelt-core/src/tracker.rs` — TrackerIssue, TrackerState types
- `crates/smelt-core/src/error.rs` — SmeltError::Tracker variant

## Expected Output

- `crates/smelt-cli/src/serve/github/mod.rs` — GhClient trait, GhIssue struct, module re-exports
- `crates/smelt-cli/src/serve/github/client.rs` — SubprocessGhClient with gh CLI wrappers
- `crates/smelt-cli/src/serve/github/mock.rs` — MockGhClient test double + unit tests
- `crates/smelt-cli/src/serve/mod.rs` — `pub mod github` added
