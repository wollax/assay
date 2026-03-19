---
id: T03
parent: S04
milestone: M004
provides:
  - just ready exits 0 with 0 warnings and 1271 tests passing
  - S04-SUMMARY.md written and committed
  - STATE.md updated: S04 ✓, M004 ✓, test count 1271
  - Race condition fix: #[serial] added to two server.rs unit tests that used set_current_dir without serialization
key_files:
  - crates/assay-mcp/src/server.rs
  - .kata/milestones/M004/slices/S04/S04-SUMMARY.md
  - .kata/STATE.md
key_decisions:
  - D061 (already present from T01/T02 — confirmed in DECISIONS.md)
patterns_established:
  - Unit tests that call std::env::set_current_dir must be marked #[serial] to avoid racing with other tests in the same binary
observability_surfaces:
  - cat .kata/STATE.md — M004 complete, 1271 tests
  - git log --oneline | grep S04 — commit history confirms slice closed
duration: ~20m
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T03: `just ready` Final Pass and Write S04-SUMMARY.md

**`just ready` exits 0 with 1271 tests passing after fixing a race condition in two server.rs unit tests that lacked `#[serial]` despite using `set_current_dir`.**

## What Happened

Ran `just ready`. The first attempt failed at `cargo fmt --all` — formatting diffs in `run.rs` (T01) and `mcp_handlers.rs` (T02) needed applying. After `cargo fmt --all`, ran `just ready` again.

The second attempt failed at `cargo test` with one test failure: `server::tests::context_diagnose_no_session_dir_returns_error` panicked with `"cannot determine working directory: No such file or directory (os error 2)"`. The test passes in isolation but races with other `#[serial]` unit tests in the same binary that also call `set_current_dir` — when a peer test's tempdir is dropped while this test is running, the process CWD becomes invalid, causing `context_diagnose` to return an `Err` instead of the expected `Ok(is_error: true)`. The same issue affected `estimate_tokens_no_session_dir_returns_error`.

Fix: added `#[serial]` to both tests in `server.rs`. This serializes them with the other `#[serial]` tests in the same binary, preventing CWD races.

Third run of `just ready` exited 0: fmt ✓, clippy 0 warnings ✓, 1271 tests ✓, deny ✓.

Appended no new decision (D061 was already present from earlier in the slice). Wrote `S04-SUMMARY.md` and updated `STATE.md` to mark M004 complete.

## Verification

- `just ready` — exits 0, 0 warnings
- `cargo test --workspace --features orchestrate 2>&1 | grep "test result"` — 1271 passed, 0 failed across all crates
- `cat .kata/milestones/M004/slices/S04/S04-SUMMARY.md | head -5` — shows `id: S04` frontmatter
- `git log --oneline -1` — shows the feat(S04) commit

## Diagnostics

- `cargo test -p assay-mcp --lib -- context_diagnose_no_session_dir_returns_error` — passes in isolation; if it fails under full parallel run, it indicates a new CWD-racing test was introduced without `#[serial]`
- `cat .kata/STATE.md` — shows M004 complete, test count 1271

## Deviations

- The `#[serial]` fix was not in the original plan but was required to achieve a green `just ready`. It is a correctness fix for pre-existing flaky tests (exposed more reliably by T02's new tests increasing parallelism), not a scope change.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` — `#[serial]` added to `context_diagnose_no_session_dir_returns_error` and `estimate_tokens_no_session_dir_returns_error`
- `crates/assay-cli/src/commands/run.rs` — `cargo fmt` applied (no logic changes)
- `crates/assay-mcp/tests/mcp_handlers.rs` — `cargo fmt` applied (no logic changes)
- `.kata/milestones/M004/slices/S04/S04-SUMMARY.md` — new file: complete slice summary
- `.kata/milestones/M004/slices/S04/S04-PLAN.md` — T03 marked `[x]`
- `.kata/STATE.md` — S04 ✓, M004 ✓, test count updated to 1271
