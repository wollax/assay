---
estimated_steps: 5
estimated_files: 2
---

# T01: PrStatusInfo type + pr_status_poll function + integration tests

**Slice:** S02 — TUI PR status panel with background polling
**Milestone:** M008

## Description

Add the `PrStatusInfo` struct and `pr_status_poll()` free function to `assay-core::pr`. This is the core data retrieval layer — it calls `gh pr view <n> --json state,statusCheckRollup,reviewDecision`, parses the JSON response, and returns a typed result. Integration tests use the established `write_fake_gh`/`with_mock_gh_path` pattern from `crates/assay-core/tests/pr.rs` to verify parsing for all status combinations.

## Steps

1. Define `PrStatusState` enum (`Open`, `Merged`, `Closed`) with `Deserialize` for lowercase JSON values from `gh`. Define `PrStatusInfo` struct with `state: PrStatusState`, `ci_pass: usize`, `ci_fail: usize`, `ci_pending: usize`, `review_decision: String`. Both types are `Debug, Clone` and live in `assay-core::pr`.
2. Implement `pr_status_poll(pr_number: u64) -> Result<PrStatusInfo>` — spawn `gh pr view <pr_number> --json state,statusCheckRollup,reviewDecision`, capture stdout, parse with `serde_json`. Map `statusCheckRollup` entries: `conclusion == "SUCCESS"` → ci_pass, `conclusion == "FAILURE"` or `conclusion == "CANCELLED"` → ci_fail, `status == "IN_PROGRESS"` or null conclusion → ci_pending. Handle empty/null `statusCheckRollup` as all-zero counts. `reviewDecision` may be `""`, `"APPROVED"`, `"CHANGES_REQUESTED"`, or `"REVIEW_REQUIRED"` — store as-is.
3. Handle error cases: `gh` not found (NotFound IO error), non-zero exit code (capture stderr, return `AssayError::Io`), malformed JSON (return `AssayError::Io`).
4. Create `crates/assay-core/tests/pr_status.rs`. Import `write_fake_gh` and `with_mock_gh_path` from existing `pr.rs` tests — if they're private, duplicate the helper functions. Write tests:
   - `test_pr_status_poll_open_with_passing_checks` — mock gh returns `{"state":"OPEN","statusCheckRollup":[{"conclusion":"SUCCESS","status":"COMPLETED","name":"CI"}],"reviewDecision":"APPROVED"}`, assert state=Open, ci_pass=1, ci_fail=0, ci_pending=0, review_decision="APPROVED"
   - `test_pr_status_poll_merged_no_checks` — state=MERGED, empty statusCheckRollup, reviewDecision=""
   - `test_pr_status_poll_open_with_mixed_checks` — 2 SUCCESS + 1 FAILURE + 1 IN_PROGRESS → ci_pass=2, ci_fail=1, ci_pending=1
   - `test_pr_status_poll_gh_not_found` — no `gh` in PATH → returns Err
   - `test_pr_status_poll_closed` — state=CLOSED
5. Run `cargo test -p assay-core --test pr_status` and `cargo clippy -p assay-core -- -D warnings`. Fix any issues.

## Must-Haves

- [ ] `PrStatusInfo` struct with state, CI counts, and review_decision fields
- [ ] `PrStatusState` enum (Open/Merged/Closed) parseable from `gh` JSON
- [ ] `pr_status_poll(pr_number) -> Result<PrStatusInfo>` calls `gh pr view --json` and parses output
- [ ] Empty/null `statusCheckRollup` handled as zero counts (no panic)
- [ ] `gh` not found returns `Err`, not panic
- [ ] 5+ integration tests pass with mock `gh` binary

## Verification

- `cargo test -p assay-core --test pr_status` — all tests pass
- `cargo clippy -p assay-core -- -D warnings` — clean

## Observability Impact

- Signals added/changed: `pr_status_poll` returns structured `AssayError::Io` on all failure paths with descriptive operation labels
- How a future agent inspects this: run `cargo test -p assay-core --test pr_status` to verify parsing correctness; read `PrStatusInfo` fields directly
- Failure state exposed: error messages include PR number and `gh` stderr output on subprocess failure

## Inputs

- `crates/assay-core/src/pr.rs` — existing module with `pr_create_if_gates_pass`, `ChunkGateFailure`, `PrCreateResult`
- `crates/assay-core/tests/pr.rs` — `write_fake_gh` and `with_mock_gh_path` helpers for mock `gh` testing pattern
- S01 summary — `Milestone.pr_number` is `Option<u64>` set during PR creation; this is the key used to decide what to poll

## Expected Output

- `crates/assay-core/src/pr.rs` — extended with `PrStatusInfo`, `PrStatusState`, `pr_status_poll()`
- `crates/assay-core/tests/pr_status.rs` — 5+ integration tests proving all parsing scenarios
