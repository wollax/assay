---
estimated_steps: 4
estimated_files: 3
---

# T02: GithubTrackerSource implementing TrackerSource

**Slice:** S03 — GitHub Issues Tracker Backend
**Milestone:** M012

## Description

Implement `GithubTrackerSource<G: GhClient>` that bridges the `GhClient` abstraction to the `TrackerSource` trait from S02. This is the core business logic: polling for ready issues, mapping `GhIssue` → `TrackerIssue`, transitioning lifecycle labels, preventing double-dispatch (D157), and auto-creating lifecycle labels (idempotent via `--force`). Generic over `G: GhClient` for testability with `MockGhClient`.

## Steps

1. Create `crates/smelt-cli/src/serve/github/source.rs` with `GithubTrackerSource<G: GhClient>`:
   - Fields: `client: G`, `repo: String`, `label_prefix: String`
   - Constructor: `new(client: G, repo: String, label_prefix: String) -> Self`
   - `ensure_labels(&self) -> Result<()>`: calls `client.create_label(repo, label)` for each `TrackerState::ALL` variant using `TrackerState::label_name(prefix)`. Logs info on creation. This is called once at startup or first poll.

2. Implement `TrackerSource for GithubTrackerSource<G: GhClient + Send + Sync>`:
   - `poll_ready_issues()`:
     - Call `self.client.auth_status()` — return `SmeltError::Tracker { operation: "poll", message }` on failure
     - Build the ready label: `TrackerState::Ready.label_name(&self.label_prefix)`
     - Call `self.client.list_issues(&self.repo, &ready_label, 50)`
     - Map each `GhIssue` to `TrackerIssue { id: number.to_string(), title, body, source_url: url }`
     - Return the vec (empty vec for no issues — not an error)
   - `transition_state(issue_id, from, to)`:
     - Parse `issue_id` to `u64` (return tracker error if not numeric)
     - Build label strings via `from.label_name(prefix)` and `to.label_name(prefix)`
     - Call `self.client.edit_labels(repo, number, vec![to_label], vec![from_label])`
     - Log info on successful transition

3. Add unit tests in `source.rs` using `MockGhClient`:
   - `test_poll_ready_issues_returns_mapped_issues` — mock returns 2 GhIssues, verify TrackerIssue mapping
   - `test_poll_ready_issues_empty_result` — mock returns empty vec, verify empty vec (not error)
   - `test_poll_ready_issues_auth_failure` — mock auth returns Err, verify SmeltError::Tracker
   - `test_transition_state_edits_labels` — mock edit_labels succeeds, verify correct label strings
   - `test_transition_state_failure_propagates` — mock edit_labels fails, verify error propagation
   - `test_ensure_labels_creates_all_lifecycle_labels` — mock create_label succeeds 6 times
   - `test_poll_uses_limit_50` — verify the ready label string is used (implicitly via mock)

4. Wire `source.rs` into `mod.rs`: add `pub mod source;` and re-export `GithubTrackerSource`.

## Must-Haves

- [ ] `GithubTrackerSource<G: GhClient>` implements `TrackerSource` from S02
- [ ] `poll_ready_issues()` checks auth, lists by ready label, maps GhIssue→TrackerIssue
- [ ] `transition_state()` swaps labels via single `edit_labels` call (D157 atomicity)
- [ ] `ensure_labels()` creates all 6 lifecycle labels idempotently
- [ ] `issue_id` parsed as u64 for gh CLI compatibility
- [ ] ≥6 unit tests covering happy path, empty poll, auth failure, transition success/failure, ensure_labels

## Verification

- `cargo test -p smelt-cli --lib -- serve::github::source` — ≥6 tests pass
- `cargo test -p smelt-cli --lib -- serve::github` — all github module tests pass
- `cargo clippy --workspace -- -D warnings` — zero warnings

## Observability Impact

- Signals added/changed: `tracing::info!` on successful label transitions with repo/issue/from/to fields; `tracing::info!` on label creation; `tracing::warn!` on auth failure
- How a future agent inspects this: check `SmeltError::Tracker { operation: "poll" }` for auth failures; check tracing output for label transition events
- Failure state exposed: Auth failures surface as structured errors before any issue listing; transition failures include the specific label names that failed

## Inputs

- `crates/smelt-cli/src/serve/github/mod.rs` — GhClient trait, GhIssue, MockGhClient from T01
- `crates/smelt-cli/src/serve/tracker.rs` — TrackerSource trait, issue_to_manifest() (consumed by caller, not here)
- `crates/smelt-core/src/tracker.rs` — TrackerIssue, TrackerState, TrackerState::ALL, TrackerState::label_name()
- `crates/smelt-core/src/error.rs` — SmeltError::Tracker

## Expected Output

- `crates/smelt-cli/src/serve/github/source.rs` — GithubTrackerSource with TrackerSource impl + unit tests
- `crates/smelt-cli/src/serve/github/mod.rs` — updated with `pub mod source` and re-export
