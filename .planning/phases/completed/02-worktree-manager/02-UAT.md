# Phase 2: Worktree Manager — UAT

**Date:** 2026-03-09
**Status:** PASSED (7/7)

## Tests

| # | Test | Expected | Status |
|---|------|----------|--------|
| 1 | `smelt init` creates `.smelt/worktrees/` | Directory created alongside config.toml | PASS |
| 2 | `smelt worktree create test-session` | Worktree + branch created, state file written | PASS |
| 3 | `smelt worktree list` shows created worktree | Table with NAME, BRANCH, STATUS, PATH columns | PASS |
| 4 | `smelt wt list` alias works | Same output as `smelt worktree list` | PASS |
| 5 | Duplicate session name produces clear error | Exit 1, error mentions "already exists" | PASS |
| 6 | `smelt worktree remove test-session --yes --force` cleans up | Worktree dir, branch, and state file all removed | PASS |
| 7 | Remove nonexistent worktree produces clear error | Exit 1, error mentions "not found" | PASS |
