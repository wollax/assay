# Phase 28: Worktree Manager — UAT

## Test Results

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | CLI help shows all 4 subcommands | PASS | create, list, status, cleanup all shown with examples |
| 2 | worktree create makes a worktree for a spec | PASS | Created at ../assay-worktrees/self-check on assay/self-check branch |
| 3 | worktree list shows created worktrees | PASS | Table with spec, branch, path columns |
| 4 | worktree status shows branch/dirty/ahead-behind | PASS | Shows branch, HEAD, clean status, ahead/behind counts |
| 5 | worktree cleanup removes a worktree | PASS | Removed worktree, list confirms empty |
| 6 | --json flag produces JSON output | PASS | Valid JSON with spec_slug, path, branch, base_branch |
| 7 | --worktree-dir overrides default path | PASS | Path correctly pointed to /tmp/assay-test-wt/self-check |
| 8 | Duplicate create returns clear error | PASS | "worktree already exists for spec" with path, exit 1 |
| 9 | Nonexistent spec returns clear error | PASS | "spec not found" with specs directory, exit 1 |

## Summary

9/9 tests passed. All worktree lifecycle operations (create, list, status, cleanup) work correctly with proper error handling, JSON output, and path overrides.

Tested: 2026-03-09
