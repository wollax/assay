---
title: "Gate module PR review suggestions"
area: assay-core
priority: low
source: PR review #28
---

# Gate Module PR Review Suggestions

Suggestions from PR #28 review (Phase 7: Gate Evaluation). None are blocking; all are quality-of-life improvements.

## Test Coverage

1. **Test pipe read error path** — No test covers the `[pipe read error: ...]` annotation path in reader threads. Could use a mock or a pipe that errors mid-read.

2. **Test thread join panic path** — No test covers the `unwrap_or_else` fallback when a reader thread panics. Could use `std::panic::catch_unwind` or a custom thread that panics.

3. **Test process group kill** — Verify that child processes spawned by a gate command are also killed on timeout (not just the direct child). E.g., `sh -c 'sleep 100 & sleep 100'` should have no lingering processes after timeout.

4. **Test truncation with multi-byte UTF-8** — Ensure `truncate_output` handles multi-byte characters at the boundary correctly (the `ceil_char_boundary` call).

5. **Test `evaluate_all` with spawn error** — `evaluate_all_captures_spawn_failure` exists but doesn't verify the error message content in stderr.

## Type Design

6. **Consider `GateRunSummary` total field** — Add a `total` field (= passed + failed + skipped) for convenience, avoiding arithmetic in consumers.

7. **Consider `CriterionResult` status enum** — Replace `Option<GateResult>` with a `CriterionStatus` enum (`Passed(GateResult)`, `Failed(GateResult)`, `Skipped`) for stronger type safety.

## Code Quality

8. **`unwrap_or_default()` on impossible None** — In `evaluate_all` line 155, `criterion.cmd.clone().unwrap_or_default()` is called in the `Err` branch where we know `cmd.is_some()` (the `None` case continues earlier). Could use `unwrap()` or restructure.

9. **`evaluate_command` doc comment on process group** — Add a note that on Unix, the child is placed in its own process group via `process_group(0)` and timeout kills the group via `killpg`.

10. **Consolidate timeout stderr formatting** — The timeout stderr message construction could be extracted to a small helper for clarity.

## Comment Accuracy

11. **Module doc async example** — The module-level doc example and the per-function doc example now both show `??` but the module-level one wraps in a `move ||` closure while the per-function one doesn't. Make them consistent.

12. **`evaluate_all` doc says "spawn failure"** — The doc comment on `evaluate_all` says "If evaluate returns an Err (spawn failure)" but `evaluate` can also return errors from `try_wait`. Widen to "execution error".
