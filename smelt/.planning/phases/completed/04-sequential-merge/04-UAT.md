# Phase 4: Sequential Merge — UAT

**Date:** 2026-03-10
**Tester:** User (manual)
**Status:** Passed

## Tests

| # | Test | Expected | Status |
|---|------|----------|--------|
| 1 | `smelt merge --help` shows usage | Displays manifest arg and --target flag | PASS |
| 2 | Clean merge of 2 sessions | Exit 0, target branch with combined work, diff stats on stdout | PASS |
| 3 | Merge conflict detected | Exit 1, conflict file list on stderr, target branch rolled back | PASS |
| 4 | Custom target branch | `--target my-branch` creates that branch name | PASS |
| 5 | No completed sessions | Exit 1, clear error message | PASS |
| 6 | Skipped failed session | Warning on stderr, other sessions still merged | PASS |

## Results

6/6 tests passed. Full workflow verified: `smelt init` → `smelt session run` → `smelt merge`.
