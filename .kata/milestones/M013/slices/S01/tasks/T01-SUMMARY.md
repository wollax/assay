---
id: T01
parent: S01
milestone: M013
provides:
  - tracing-test dev-dependency wired into assay-backends
  - Q001 contract tests: two #[traced_test] tests asserting warn on empty/missing-slash repo
  - Q002 contract test: one test asserting Err when .github-issue-number contains "0"
  - All three new tests fail in expected ways, proving the contracts are meaningful
key_files:
  - crates/assay-backends/Cargo.toml
  - crates/assay-backends/tests/github_backend.rs
key_decisions:
  - Q002 tested indirectly via read_run_state (calls read_issue_number internally); read_issue_number remains private
  - Q002 failure mode: current code calls gh with issue 0, returns gh API error — not an early parse rejection; T02 must add a zero-guard in read_issue_number
patterns_established:
  - tracing-test #[traced_test] + logs_contain() pattern for warn assertion tests in assay-backends
observability_surfaces:
  - none (tests only; no runtime signal changes)
duration: 15min
verification_result: passed
completed_at: 2026-03-28T00:57:00Z
blocker_discovered: false
---

# T01: Write contract tests for Q001–Q004

**Three contract tests added (Q001 ×2, Q002 ×1) that fail in expected ways, plus `tracing-test` wired as a dev-dep of `assay-backends`.**

## What Happened

Added `tracing-test = { workspace = true }` to `[dev-dependencies]` in `crates/assay-backends/Cargo.toml`. Added the `tracing_test::traced_test` import to the integration test file, then appended three new test functions:

- `test_new_warns_on_empty_repo` — `#[traced_test]`, constructs `GitHubBackend::new("", None)`, asserts `logs_contain("malformed")`. Fails because no warn is emitted yet.
- `test_new_warns_on_repo_missing_slash` — same pattern for `"noslash"`. Fails for the same reason.
- `test_read_issue_number_rejects_zero` — writes `"0"` to `.github-issue-number`, calls `read_run_state` (which calls the private `read_issue_number` internally). Fails because the current code passes 0 directly to `gh issue view` rather than rejecting it early — the error returned is a GitHub API error, not the zero-guard `Err` the test expects.

The indirect testing approach for Q002 (via `read_run_state`) was confirmed correct since `read_issue_number` is private. T02 should add an explicit `if number == 0` guard in `read_issue_number` that returns `Err` mentioning the value and path.

## Verification

```
cargo test -p assay-backends --features github
```

- Compiles without errors ✓
- Existing 8 tests: all pass ✓
- `test_new_warns_on_empty_repo`: FAILED — "expected warn log containing 'malformed'" ✓ (expected)
- `test_new_warns_on_repo_missing_slash`: FAILED — same ✓ (expected)
- `test_read_issue_number_rejects_zero`: FAILED — error message is a `gh` API error, not a zero-guard error ✓ (expected)

All failures are meaningful contract failures, not accidental ones.

## Diagnostics

- Run `cargo test -p assay-backends --features github -- --nocapture` to see test output with tracing logs
- Q001 test output: `#[traced_test]` subscriber captures no warn lines → assertion fails
- Q002 test output: stderr shows `gh issue view failed ... GraphQL: Could not resolve to a Repository ...` — proves issue 0 reaches the API instead of being rejected early

## Deviations

- Q004 was noted as "no test needed, just a grep verification" — confirmed no test function was written; the grep command is already in the slice plan verification section.

## Known Issues

- Q002: The current failure mode is a `gh` API error (GraphQL repo-not-found), not an early `Err` from a zero-guard. T02 must add `if number == 0 { return Err(...) }` inside `read_issue_number` to make the contract test pass with the correct error shape.

## Files Created/Modified

- `crates/assay-backends/Cargo.toml` — added `tracing-test = { workspace = true }` to `[dev-dependencies]`
- `crates/assay-backends/tests/github_backend.rs` — added `use tracing_test::traced_test` import; added `test_new_warns_on_empty_repo`, `test_new_warns_on_repo_missing_slash`, `test_read_issue_number_rejects_zero`
