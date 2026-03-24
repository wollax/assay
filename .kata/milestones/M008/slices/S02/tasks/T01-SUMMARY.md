---
id: T01
parent: S02
milestone: M008
provides:
  - PrStatusState enum (Open/Merged/Closed) with serde Deserialize
  - PrStatusInfo struct with state, ci_pass, ci_fail, ci_pending, review_decision
  - pr_status_poll(pr_number) -> Result<PrStatusInfo> calling gh pr view --json
  - parse_pr_status_json internal parser handling all check conclusion variants
  - 8 integration tests covering all status combinations and error paths
key_files:
  - crates/assay-core/src/pr.rs
  - crates/assay-core/tests/pr_status.rs
key_decisions:
  - "PrStatusState uses serde rename_all=UPPERCASE to match gh JSON directly"
  - "CANCELLED conclusion counted as ci_fail alongside FAILURE"
  - "review_decision stored as raw String (not enum) — gh may add values in future"
patterns_established:
  - "pr_status_poll follows same AssayError::Io pattern as pr_create_if_gates_pass"
  - "write_fake_gh + with_mock_gh_path pattern duplicated in pr_status tests (helpers are test-private)"
observability_surfaces:
  - "AssayError::Io on all failure paths includes PR number and descriptive operation label"
  - "gh stderr forwarded in error message on non-zero exit"
duration: 10min
verification_result: passed
completed_at: 2026-03-23T12:00:00Z
blocker_discovered: false
---

# T01: PrStatusInfo type + pr_status_poll function + integration tests

**Added `PrStatusInfo`/`PrStatusState` types and `pr_status_poll()` function that shells out to `gh pr view --json` and parses CI check counts, PR state, and review decision**

## What Happened

Extended `assay-core::pr` with the data retrieval layer for PR status polling. Added `PrStatusState` enum (Open/Merged/Closed) with serde `UPPERCASE` rename for direct gh JSON deserialization, and `PrStatusInfo` struct carrying state, CI pass/fail/pending counts, and review_decision string.

The `pr_status_poll(pr_number)` function spawns `gh pr view <n> --json state,statusCheckRollup,reviewDecision`, parses the output via internal `RawPrStatus`/`RawStatusCheck` structs, and maps check conclusions: SUCCESS → ci_pass, FAILURE/CANCELLED → ci_fail, null/IN_PROGRESS → ci_pending. Empty or null `statusCheckRollup` arrays produce zero counts without panic.

All error paths return `AssayError::Io` with descriptive operation labels including the PR number, consistent with the existing D065 pattern.

## Verification

- `cargo test -p assay-core --test pr_status` — 8 tests pass: OPEN+passing, MERGED+empty, OPEN+mixed, gh-not-found, CLOSED+null-rollup, non-zero exit, malformed JSON, CANCELLED-as-failure
- `cargo clippy -p assay-core -- -D warnings` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — clean

## Diagnostics

Run `cargo test -p assay-core --test pr_status` to verify all parsing scenarios. Errors from `pr_status_poll` include the PR number and gh stderr in the error chain.

## Deviations

- Added 3 extra tests beyond the 5 required (non-zero exit, malformed JSON, CANCELLED conclusion) for better coverage — no plan conflict.
- Used `write_fake_gh_stderr` helper for non-zero exit test (writes to stderr instead of stdout).

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/pr.rs` — Added `PrStatusState`, `PrStatusInfo`, `pr_status_poll()`, `parse_pr_status_json()`, `RawPrStatus`, `RawStatusCheck`
- `crates/assay-core/tests/pr_status.rs` — 8 integration tests for all status parsing scenarios and error paths
