# S03: GitHub Issues Tracker Backend — UAT

**Milestone:** M012
**Written:** 2026-03-28

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: The slice's verification contract is unit tests with `MockGhClient` (no real `gh` required) plus gated integration tests for real `gh` CLI interaction. All 24 unit tests pass in CI; real `gh` CLI is exercised by gated tests requiring a human-configured environment. No live `smelt serve` dispatch loop exists yet (that's S05), so live-runtime UAT is deferred.

## Preconditions

**For unit tests (no `gh` required):**
- Rust toolchain installed
- `cargo test --workspace` passes from a clean checkout

**For gated integration tests (real `gh` CLI):**
- `gh` CLI installed and authenticated (`gh auth status` returns 0)
- `SMELT_GH_TEST=1` set in environment
- `SMELT_GH_REPO=owner/repo` set to a real accessible repository

## Smoke Test

```
cargo test -p smelt-cli --lib -- serve::github
```

Expected: `16 passed; 0 failed; 2 ignored` (the 2 ignored are gated integration tests).

## Test Cases

### 1. Unit test suite passes (no gh binary required)

```bash
cargo test -p smelt-cli --lib -- serve::github
```

**Expected:** 16 tests pass, 2 ignored (integration tests), 0 failed.

### 2. Config validation rejects missing/invalid repo

```bash
cargo test -p smelt-cli --lib -- serve::config
```

**Expected:** 17 tests pass. Specifically:
- `test_tracker_github_requires_repo` — missing repo fails validation
- `test_tracker_github_repo_empty_string` — empty string fails validation
- `test_tracker_github_invalid_repo_format` — `no-slash` and `a/b/c` fail validation
- `test_tracker_github_valid_repo` — `owner/repo` passes validation
- `test_tracker_linear_ignores_repo` — linear provider passes without repo

### 3. Full workspace regression check

```bash
cargo test --workspace
```

**Expected:** 360+ tests pass, 0 failed, ~11 ignored. Zero regressions vs pre-S03 baseline.

### 4. Zero lint warnings

```bash
cargo clippy --workspace -- -D warnings
cargo doc --workspace --no-deps
```

**Expected:** Both exit 0 with no warnings or errors.

### 5. Gated integration tests against real gh CLI

```bash
SMELT_GH_TEST=1 SMELT_GH_REPO=owner/repo cargo test -p smelt-cli -- --ignored github
```

**Expected:** `test_gh_auth_status_integration` and `test_gh_list_issues_integration` pass. The auth test confirms `gh auth status` exits 0. The list test confirms `gh issue list` returns parseable JSON (empty list is valid).

## Edge Cases

### Missing gh binary

In a test environment without `gh` in PATH, `SubprocessGhClient::auth_status()` returns `SmeltError::Tracker { operation: "gh_binary", message: "gh binary not found: ..." }`. This is exercised indirectly by `test_subprocess_gh_client_binary_discovery` in the unit test suite.

### Invalid issue_id (non-numeric)

`GithubTrackerSource::transition_state("not-a-number", ...)` returns `SmeltError::Tracker { operation: "transition", message: "... not a valid u64" }`. Exercised by `test_transition_state_invalid_issue_id`.

### Auth failure during poll

When `MockGhClient::auth_status()` returns an error, `poll_ready_issues()` wraps it as `SmeltError::Tracker { operation: "poll" }` and returns immediately without calling `list_issues()`. Exercised by `test_poll_ready_issues_auth_failure`.

### Empty poll result

When `list_issues()` returns an empty vec, `poll_ready_issues()` returns `Ok(vec![])`. The caller (S05's TrackerPoller) should handle empty results gracefully and wait for the next poll interval. Exercised by `test_poll_ready_issues_empty_result`.

## Failure Signals

- `cargo test -p smelt-cli --lib -- serve::github` returning any `FAILED` lines indicates a regression in the GhClient/GithubTrackerSource implementation.
- `cargo clippy --workspace -- -D warnings` producing warnings indicates a new lint issue introduced.
- `cargo doc --workspace --no-deps` producing warnings indicates a documentation gap.
- Gated integration tests failing with "permission denied" or non-zero exit → `gh` auth expired; run `gh auth login`.
- Gated integration tests failing with "repository not found" → `SMELT_GH_REPO` points to a non-existent or inaccessible repo.

## Requirements Proved By This UAT

- **R070** (partially) — `GithubTrackerSource` implements `TrackerSource`: auth-gated polling, `GhIssue` → `TrackerIssue` mapping, atomic label transitions (D157), manifest generation via `issue_to_manifest()`. Full end-to-end proof deferred to S05.
- **R074** (partially, GitHub side) — Label-based lifecycle proven: `ensure_labels()` creates all 6 lifecycle labels, `transition_state()` swaps labels atomically via single `gh issue edit` call, `poll_ready_issues()` filters by ready label.

## Not Proven By This UAT

- **R070 end-to-end:** `smelt serve` with `[tracker]` section picking up real GitHub Issues and dispatching Assay sessions — requires S05 (TrackerPoller integration into dispatch loop).
- **R074 Linear side:** Linear label lifecycle — requires S04.
- **R075** (state backend passthrough): `state_backend` field serialized into Assay RunManifest — requires S05.
- **Live `gh` issue lifecycle:** Real issue progressing through all 5 lifecycle labels (`smelt:ready → smelt:queued → smelt:running → smelt:pr-created → smelt:done`) — requires S05 full integration test.
- **Double-dispatch race condition under load:** D157 atomicity holds under concurrent `smelt serve` instances — single subprocess call minimizes the window but distributed locking is not proven.
- **`smelt serve` TUI display** of tracker-sourced jobs — requires S05.

## Notes for Tester

- The 2 ignored tests (`test_gh_auth_status_integration`, `test_gh_list_issues_integration`) are safe to run against any repo you have read access to. An empty issue list is valid — the test just verifies the JSON parses without error.
- `SubprocessGhClient` uses `-R owner/repo` on every `gh` command — it never infers the repo from the current working directory. This means the integration tests work correctly from any directory.
- All 16 unit tests run in ~10ms with no external dependencies. The gated tests require `gh` auth and take ~1-2s each (network roundtrip to GitHub API).
