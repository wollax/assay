# Phase 36: Correctness & Robustness — UAT

## Tests

| # | Area | Test | Status |
|---|------|------|--------|
| 1 | Worktree | WorktreeStatus.ahead/behind are nullable in schema (`["integer", "null"]`) | PASS |
| 2 | Worktree | create() writes .assay/worktree.json with base_branch (roundtrip verified) | PASS |
| 3 | Worktree | status() returns None ahead/behind + warning when metadata missing | PASS |
| 4 | Worktree | list() populates base_branch from metadata | PASS |
| 5 | Worktree | fetch param accepted by worktree_status tool (Option<bool>, default false) | PASS |
| 6 | Session | Not-found errors include recovery hints (gate_run, gate_history) | PASS |
| 7 | Session | gate_report and gate_finalize error format is consistent | PASS |
| 8 | Session | timed_out_sessions has MAX_TIMED_OUT_ENTRIES=100 cap with eviction | PASS |
| 9 | Diff | AgentSession has diff, diff_truncated, diff_bytes_original fields | PASS |
| 10 | Diff | truncate_diff returns None for empty, handles within/over budget | PASS |
| 11 | Diff | DIFF_BUDGET_BYTES is 32 KiB (32 * 1024), used in gate_run handler | PASS |

## Result

**11/11 tests passed** — UAT complete 2026-03-11
