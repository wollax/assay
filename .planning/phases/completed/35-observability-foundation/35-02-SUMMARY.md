# Phase 35 Plan 02: Outcome Filter and Limit Cap Summary

**One-liner:** Added `outcome` filter parameter (passed/failed/any) to `gate_history` and capped `limit` at 50, with load-and-filter loop iterating newest-first.

## Changes

### server.rs — Outcome filter and limit cap

- Added `outcome: Option<String>` field to `GateHistoryParams` with schemars description
- Capped `limit` at 50 via `.min(50)` (default remains 10)
- Refactored list mode from take-then-load to load-and-filter loop iterating newest-first
- Outcome matching: `"passed"` requires `required_failed == 0`, `"failed"` requires `required_failed > 0`, `"any"` or unrecognized returns all
- `total_runs` reflects total on-disk records, not filtered count
- Updated tool description to mention outcome filter

### Integration tests (mcp_handlers.rs)

- Added `run_gate` and `query_history` test helpers
- `gate_history_outcome_failed_filters_correctly` — verifies only failed runs returned
- `gate_history_outcome_passed_filters_correctly` — verifies only passed runs returned
- `gate_history_outcome_any_returns_all` — verifies any and default both return all
- `gate_history_limit_capped_at_50` — verifies limit=100 accepted, capped internally
- `gate_history_default_limit_is_10` — creates 15 runs, verifies 10 returned by default

## Deviations from Plan

| # | Type | Description |
|---|------|-------------|
| 1 | Auto-fix | Unit test in server.rs also referenced `GateHistoryParams` without the new `outcome` field — fixed in separate commit |

## Decisions Made

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Unrecognized outcome values treated as "any" | Graceful degradation — no error for typos, just returns all runs |
| 2 | `total_runs` not filtered | Reflects on-disk count so agents can understand the full history size |

## Commits

- `21225bf`: feat(35-02): add outcome filter and limit cap to gate_history
- `9a0ff72`: test(35-02): add integration tests for outcome filter and limit cap
- `8cd9c1b`: fix(35-02): add missing outcome field to unit test GateHistoryParams
