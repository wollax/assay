---
id: S01
parent: M013
milestone: M013
provides:
  - Q001: tracing::warn! in GitHubBackend::new for empty or slash-missing repo
  - Q002: zero-guard in read_issue_number rejects issue number 0
  - Q003: GhRunner::gh_error helper extracted; 3 call sites deduplicated
  - Q004: factory.rs doc comment cleaned of (M011/S02), (M011/S03), (M011/S04)
  - Contract tests for Q001 (×2) and Q002 (×1) using tracing-test
requires: []
affects:
  - S02 (independent)
  - S03 (independent)
  - S04 (independent)
key_files:
  - crates/assay-backends/src/github.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-backends/tests/github_backend.rs
  - crates/assay-backends/Cargo.toml
key_decisions:
  - Q002 tested indirectly via read_run_state (read_issue_number is private)
  - Q002 failure mode confirmed: without guard, gh receives issue 0 and returns GraphQL API error — zero-guard must be in read_issue_number
  - from_utf8_lossy count is 2, not 1: gh_error uses it for stderr; create_issue uses it for stdout URL parsing — both correct, plan was overspecified
  - GhRunner::gh_error consolidates warn + error construction for all gh CLI failures
patterns_established:
  - tracing-test #[traced_test] + logs_contain() pattern for warn assertions in assay-backends
  - GhRunner::gh_error as the single place for stderr-decode + warn + AssayError construction
observability_surfaces:
  - tracing::warn! at construction time for malformed repo (field: repo), visible under RUST_LOG=assay_backends=warn
  - read_issue_number Err includes "0" and path for diagnosability
drill_down_paths:
  - .kata/milestones/M013/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M013/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M013/slices/S01/tasks/T03-SUMMARY.md
duration: 30min
verification_result: passed
completed_at: 2026-03-28
---

# S01: GitHubBackend correctness fixes (Q001–Q004)

**Four correctness fixes shipped — malformed-repo warn, issue-0 rejection, gh_error extraction, and factory doc cleanup — all proven by 11 passing contract tests and a green `just ready`.**

## What Happened

**T01** wired `tracing-test` as a dev-dependency in `assay-backends` and added three failing contract tests: two for Q001 (warn on empty repo / warn on missing-slash repo using `#[traced_test]` + `logs_contain("malformed")`) and one for Q002 (reject issue number 0 via `read_run_state` since `read_issue_number` is private). All three failed in the expected ways, confirming the contracts were meaningful and not vacuous.

**T02** implemented all four fixes in order:
- Q003 first: extracted `GhRunner::gh_error(&self, operation, output) -> AssayError` that handles `from_utf8_lossy(stderr)` → `tracing::warn!` → `AssayError::io` construction, then replaced three duplicated error blocks in `create_issue`, `create_comment`, and `get_issue_json` with single calls to `self.gh_error(...)`.
- Q001: added repo validation in `GitHubBackend::new` — emits `tracing::warn!(repo, "malformed GitHub repo — expected 'owner/repo' format")` when `repo.is_empty() || !repo.contains('/')`. Constructor stays infallible per D177.
- Q002: added `if number == 0` guard in `read_issue_number` immediately after the `parse::<u64>()` call, returning `Err(AssayError::io("invalid issue number: 0 (possible file corruption)", &path, ...))`.
- Q004: removed `(M011/S02)`, `(M011/S03)`, `(M011/S04)` parentheticals from the `backend_from_config` doc comment in `factory.rs`.

All 11 tests passed after T02. One deviation from the plan: `from_utf8_lossy` appears twice (once in `gh_error` for stderr, once in `create_issue` for stdout URL parsing) — both are correct; the plan's grep-count of 1 was slightly overspecified.

**T03** ran `just ready` — all 1501 tests passed, zero clippy warnings, `cargo deny` clean.

## Verification

- `cargo test -p assay-backends --features github` — 11 passed, 0 failed ✓
- `just ready` exits 0 with 1501 tests ✓
- `grep -c '(M011/S' crates/assay-backends/src/factory.rs` → 0 ✓
- Q001 `test_new_warns_on_empty_repo` passes (`logs_contain("malformed")`) ✓
- Q001 `test_new_warns_on_repo_missing_slash` passes ✓
- Q002 `test_read_issue_number_rejects_zero` passes (Err before gh call) ✓
- Q003 `gh_error` used by all 3 methods ✓
- `from_utf8_lossy` count is 2 (gh_error stderr + create_issue stdout — both correct) ✓

## Requirements Advanced

- R081 (GitHubBackend construction validation) — all four sub-items Q001–Q004 implemented and proven by contract tests

## Requirements Validated

- R081 — construction-time warn (Q001), zero-guard rejection (Q002), gh_error extraction (Q003), and factory doc cleanup (Q004) all proven by 11 passing tests and `just ready` green. R081 moves from active → validated.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- `from_utf8_lossy` count is 2 instead of the 1 predicted in the plan. The second occurrence is in `create_issue` for stdout URL parsing (extracting the new issue URL from gh output), which is unrelated to the error-handling duplication that Q003 targets. The plan's grep check was overspecified. Both occurrences are correct.
- Test count is 1501, not 1529+ as predicted. The 1529 figure was from earlier milestone estimates; 1501 is the current nextest baseline. No tests were lost.

## Known Limitations

- Real `gh` CLI validation is UAT-only — all contract tests mock the subprocess environment or test upstream of the CLI call.
- Q001 warn tests require `#[traced_test]` + `logs_contain()` — they do not verify the warn is emitted in production binaries without a tracing subscriber configured.

## Follow-ups

- none — all Q001–Q004 items fully resolved

## Files Created/Modified

- `crates/assay-backends/src/github.rs` — Q001 warn in `new()`, Q002 zero-guard in `read_issue_number`, Q003 `gh_error` helper with 3 call sites
- `crates/assay-backends/src/factory.rs` — removed milestone identifiers from doc comment
- `crates/assay-backends/tests/github_backend.rs` — added 3 contract tests (2 × Q001, 1 × Q002), `use tracing_test::traced_test`
- `crates/assay-backends/Cargo.toml` — added `tracing-test = { workspace = true }` to `[dev-dependencies]`

## Forward Intelligence

### What the next slice should know
- S01 is a leaf slice with no new API surface — nothing downstream depends on the changes made here.
- The `tracing-test` dev-dep pattern is now established in `assay-backends`; S02/S03/S04 can reuse it if needed.

### What's fragile
- Q001 warn tests depend on `tracing-test` capturing logs in the same thread — if test execution becomes async or moves to a different runtime model, the subscriber capture may not fire. Low risk for current test setup.

### Authoritative diagnostics
- `RUST_LOG=assay_backends=warn cargo test -p assay-backends --features github -- --nocapture` — shows warn events from malformed-repo construction in test output
- `grep -n "gh_error" crates/assay-backends/src/github.rs` — confirms all error paths route through the helper

### What assumptions changed
- Plan assumed `from_utf8_lossy` would appear exactly once after Q003. In reality it appears twice: once for stderr in `gh_error` and once for stdout URL parsing in `create_issue`. The stdout use is unrelated to Q003 and correct.
