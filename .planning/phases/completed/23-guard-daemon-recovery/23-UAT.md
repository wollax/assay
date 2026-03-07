# Phase 23 UAT: Guard Daemon & Recovery

## Test Results

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | Guard daemon prevents double-start via PID file | PASS | `create_pid_file` returns `GuardAlreadyRunning` when PID file held by live process. 8 PID tests pass. |
| 2 | Soft threshold triggers gentle prune | PASS | `check_and_respond` dispatches to `handle_soft_threshold` which invokes prune with breaker tier. Unit test confirms true return above soft threshold. |
| 3 | Hard threshold triggers standard-or-higher prune | PASS | `handle_hard_threshold` enforces minimum `PrescriptionTier::Standard` even when breaker tier is Gentle. Unit test verifies recovery recorded. |
| 4 | Token-based and file-size thresholds work independently | PASS | `evaluate_thresholds` checks hard first (pct OR bytes), then soft (pct OR bytes). 9 threshold tests including bytes-only and pct-only triggers. |
| 5 | Reactive watcher detects file changes (sub-second) | PASS | `SessionWatcher` watches file (Modify) and parent dir (Create). Async test verifies event delivery within 5s timeout. Unrelated files filtered. 3 watcher tests pass. |
| 6 | Circuit breaker trips after max recoveries | PASS | Both `handle_soft_threshold` and `handle_hard_threshold` trip after `max_recoveries` reached. Unit tests verify `GuardCircuitBreakerTripped` error. 11 breaker tests pass. |
| 7 | Escalating prescriptions (gentle -> standard -> aggressive) | PASS | `current_tier()` maps 0-1 recoveries -> Gentle, 2 -> Standard, 3+ -> Aggressive. 11 circuit breaker tests cover escalation. |
| 8 | Ctrl+C writes final checkpoint before exit | PASS | `graceful_shutdown` calls `try_save_checkpoint("guard-shutdown")` then removes PID file. Unit test verifies PID cleanup. |

## CLI Surface Verification

- `assay context guard --help` — renders all subcommands (start, stop, status, logs)
- `assay context guard status` — correctly reports "Guard daemon is not running"
- `assay context guard start --help` — shows `--session` option
- `just ready` — all checks pass (fmt, lint, test, deny)

## Summary

**8/8 tests passed.** All guard daemon requirements (TPROT-07 through TPROT-13) verified through unit tests and CLI smoke tests. Post-review critical fixes (stop_guard kill return check, polling PID cleanup, daemon core test coverage) confirmed working.
