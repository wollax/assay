---
id: T03
parent: S03
milestone: M012
provides:
  - TrackerConfig.repo field with serde(default) deserialization
  - GitHub provider repo validation (owner/repo format, required, non-empty)
  - Linear provider ignores repo (no validation)
  - 5 new config unit tests for repo validation scenarios
  - 2 gated integration tests exercising SubprocessGhClient against real gh CLI
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/github/mod.rs
  - crates/smelt-cli/src/serve/tracker.rs
key_decisions:
  - "Repo validation uses slash-count check (exactly one '/') — simple, covers owner/repo without regex overhead"
patterns_established:
  - "Integration test gate pattern: SMELT_GH_TEST=1 + SMELT_GH_REPO env vars with eprintln skip in github/mod.rs"
observability_surfaces:
  - "Validation error messages include repo field context: 'repo must be set', 'repo must not be empty', 'repo must be in owner/repo format'"
duration: 10min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T03: TrackerConfig.repo field, validation, and integration tests

**Added `repo: Option<String>` to TrackerConfig with GitHub-specific owner/repo validation and 2 gated gh CLI integration tests**

## What Happened

Added `repo: Option<String>` with `#[serde(default)]` to `TrackerConfig`. When `provider == "github"`, validation requires repo to be `Some`, non-empty, and contain exactly one `/` (owner/repo format). When `provider == "linear"`, repo is ignored entirely. Validation errors are collected per D018 alongside existing tracker errors.

Added 5 new config unit tests: `test_tracker_github_requires_repo`, `test_tracker_github_invalid_repo_format` (no-slash and multi-slash cases), `test_tracker_github_valid_repo`, `test_tracker_linear_ignores_repo`, and `test_tracker_github_repo_empty_string`. Updated existing tests that use `provider = "github"` to include `repo = "owner/repo"` so they test their intended error in isolation.

Added 2 `#[ignore]` integration tests in `github/mod.rs` gated by `SMELT_GH_TEST=1` and `SMELT_GH_REPO`: `test_gh_auth_status_integration` and `test_gh_list_issues_integration`. Both skip gracefully with eprintln when env vars are not set.

## Verification

- `cargo test -p smelt-cli --lib -- serve::config` — 17 tests passed (5 new + 12 existing)
- `cargo test --workspace` — 360 passed, 0 failed, 11 ignored (zero regressions)
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Diagnostics

- Invalid repo format at startup → `"invalid tracker configuration"` error with `"repo must be in owner/repo format"` or `"repo must be set"` substring
- Integration tests inspectable via: `SMELT_GH_TEST=1 SMELT_GH_REPO=owner/repo cargo test -p smelt-cli -- --ignored github`

## Deviations

- Updated existing config tests to include `repo = "owner/repo"` — necessary to prevent cascading validation errors from the new repo requirement. Updated `with_tracker_toml` helper and `make_tracker_config` in tracker.rs as well.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — Added `repo: Option<String>` to TrackerConfig, repo validation in validate(), 5 new tests, updated existing tests
- `crates/smelt-cli/src/serve/github/mod.rs` — Added 2 gated integration tests for SubprocessGhClient
- `crates/smelt-cli/src/serve/tracker.rs` — Updated make_tracker_config() test helper to include repo field
