# Phase 18 Plan 02: Enforcement Surface & CLI Output Summary

Enforcement-aware streaming output with warned counter, advisory labels, and yellow WARN display for advisory failures.

## Tasks Completed

| # | Task | Commit |
|---|------|--------|
| 1 | Add warned counter and enforcement-aware streaming | `97288c0` |
| 2 | Update summary line with warned category | `1100cbc` |
| 3 | Verify full build, tests, and enforcement behavior | `a9649ab` (fmt fix) |

## Key Changes

- `StreamCounters` gained `warned: usize` field
- `stream_criterion()` now accepts `gate_section` parameter and resolves enforcement inline
- Advisory failures increment `warned` (not `failed`) and display as yellow `WARN`
- Advisory criteria (pass or fail) show `[advisory]` prefix in output
- Required failures remain red `FAILED` (unchanged behavior)
- Summary line format: `N passed, M failed, K warned, J skipped (of T total)`
- Exit code driven by `counters.failed > 0` (streaming) or `enforcement.required_failed > 0` (JSON)
- Removed post-hoc `has_required_failure` tracking — no longer needed since `counters.failed` is required-only

## Deviations

None — plan executed exactly as written.

## Verification

- `just ready` passes (fmt, clippy, tests, deny)
- Code review confirms advisory failures do not affect exit code
- Code review confirms summary arithmetic: passed + failed + warned + skipped == total
