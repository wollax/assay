# Phase 15 Verification: Run History CLI

**Status:** passed
**Date:** 2026-03-05
**Verified by:** kata-verifier (automated)

## Must-Haves Verification

### HIST-02: User can view recent gate run history via `assay history <spec>`

| # | Check | Status |
|---|-------|--------|
| 1 | `GateCommand::History` variant exists with name, run_id, last, json, limit fields | PASS |
| 2 | `handle_gate_history()` displays table with columns: #, Timestamp, Status, Passed, Failed, Skipped, Req Failed, Adv Failed, Duration | PASS |
| 3 | `handle_gate_history_detail()` displays formatted detail view with all GateRunRecord fields | PASS |
| 4 | `--last` flag resolves to most recent run ID via `list().last()` | PASS |
| 5 | `--json` flag outputs JSON for both table and detail views | PASS |
| 6 | `--limit N` controls table row count (default 20) | PASS |
| 7 | Empty history prints "No history for <spec>" and exits 0 | PASS |
| 8 | Unknown spec prints error and exits 1 | PASS |

### HIST-03: Configurable retention policy enforced on save

| # | Check | Status |
|---|-------|--------|
| 1 | `GatesConfig.max_history: Option<usize>` with serde(default) | PASS |
| 2 | `save()` accepts `max_history` parameter and returns `SaveResult` | PASS |
| 3 | `prune()` removes oldest files when count exceeds limit | PASS |
| 4 | `Some(0)` and `None` skip pruning | PASS |
| 5 | Prune tests: removes_oldest, zero_unlimited, none_no_pruning, pruned_count | PASS |

### Gate run save wiring

| # | Check | Status |
|---|-------|--------|
| 1 | `save_run_record()` helper called from all gate run paths | PASS |
| 2 | `handle_gate_run()` streaming path saves history | PASS |
| 3 | `handle_gate_run()` JSON path saves history with full summary | PASS |
| 4 | `handle_gate_run_all()` saves per-spec history | PASS |
| 5 | Prune messages on stderr, suppressed with --json | PASS |
| 6 | Save failures are warnings, not fatal | PASS |

## Test Suite

- 212 tests pass, 3 ignored, 0 failed
- `just ready` passes clean (fmt-check + lint + test + deny)

## Score: 14/14 must-haves verified
