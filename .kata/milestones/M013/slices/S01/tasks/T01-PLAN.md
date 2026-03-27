---
estimated_steps: 4
estimated_files: 3
---

# T01: Write contract tests for Q001–Q004

**Slice:** S01 — GitHubBackend correctness fixes (Q001–Q004)
**Milestone:** M013

## Description

Test-first: define the expected behavior for all four correctness fixes before implementing them. Add `tracing-test` as a dev-dependency of `assay-backends` so Q001 warn assertions can use `#[traced_test]` + `logs_contain()`. Write new test functions that will initially fail or not compile, proving the contracts are meaningful.

## Steps

1. Add `tracing-test = { workspace = true }` to `[dev-dependencies]` in `crates/assay-backends/Cargo.toml`.
2. Add Q001 tests to `crates/assay-backends/tests/github_backend.rs`:
   - `test_new_warns_on_empty_repo`: construct `GitHubBackend::new("".into(), None)`, assert `logs_contain("malformed")` or similar warn text.
   - `test_new_warns_on_repo_missing_slash`: construct `GitHubBackend::new("noslash".into(), None)`, assert `logs_contain("malformed")`.
   Both use `#[traced_test]` attribute. Neither needs `#[serial]` (no PATH mutation).
3. Add Q002 test:
   - `test_read_issue_number_rejects_zero`: write `"0"` to a `.github-issue-number` file in a tempdir, call `GitHubBackend::read_issue_number(&run_dir)`, assert it returns `Err` with a message mentioning `"0"`.
   Note: `read_issue_number` is a private method. Test it indirectly via `read_run_state` on a run_dir with `"0"` in the issue file — or make it `pub(crate)` for testing. Research shows the existing test pattern accesses `GitHubBackend` public API only. Use `read_run_state` which calls `read_issue_number` internally — the error propagates.
4. Add Q004 verification note in the slice plan (already covered by `grep` command — no test function needed, just the verification command).

## Must-Haves

- [ ] `tracing-test` added as dev-dependency of `assay-backends`
- [ ] Q001 tests: two tests asserting `tracing::warn!` on empty and slash-missing repo
- [ ] Q002 test: one test asserting `Err` when issue number file contains `"0"`
- [ ] All new tests target real behavioral contracts, not implementation details

## Verification

- `cargo test -p assay-backends --features github` compiles (new tests may fail — that's expected)
- Q001 tests fail with "assert logs_contain failed" (no warn emitted yet) — confirms the test is meaningful
- Q002 test fails with assertion error (0 is currently accepted) — confirms the test is meaningful
- Existing 8 tests still pass unchanged

## Observability Impact

- Signals added/changed: None (tests only)
- How a future agent inspects this: `cargo test -p assay-backends --features github -- --nocapture` shows test output
- Failure state exposed: None

## Inputs

- `crates/assay-backends/tests/github_backend.rs` — existing 8 contract tests with helpers (`write_mock_gh`, `with_mock_gh_path`, `make_backend`, `sample_status`)
- `crates/assay-backends/Cargo.toml` — current dev-dependencies (needs `tracing-test` added)
- `Cargo.toml` (workspace root) — `tracing-test` already defined as workspace dep with `no-env-filter` feature

## Expected Output

- `crates/assay-backends/Cargo.toml` — `tracing-test` added to `[dev-dependencies]`
- `crates/assay-backends/tests/github_backend.rs` — 3+ new test functions for Q001 (×2) and Q002 (×1)
