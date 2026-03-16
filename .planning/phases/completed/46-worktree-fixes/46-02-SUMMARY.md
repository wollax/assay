---
phase: 46-worktree-fixes
plan: 02
subsystem: worktree
tags: [error-handling, default-branch, detection, actionable-errors]
depends_on: [46-01]
files_modified:
  - crates/assay-core/src/worktree.rs
decisions:
  - "Reuse WorktreeGitFailed variant with custom stderr — no new error variants"
  - "Error message names three remediation options: git remote set-head, init.defaultBranch, base_branch parameter"
  - "Single error path — no distinction between 'no remote' and 'remote HEAD not configured'"
metrics:
  tasks_completed: 2
  tasks_total: 2
  deviations: 0
  duration: "~3 min"
---

# Phase 46 Plan 02: Fallible Default Branch Detection Summary

## Objective

Change `detect_default_branch()` from infallible (`-> String`) to fallible (`-> Result<String>`) so repos without a configured remote HEAD produce an actionable error instead of silently falling back to `"main"`.

## What Changed

### Task 1: Make detect_default_branch() return Result

Changed `detect_default_branch()` signature from `-> String` to `-> Result<String>` in `crates/assay-core/src/worktree.rs` (lines 45-62):

- Removed `.unwrap_or_else(|| "main".to_string())` silent fallback
- Replaced with `.ok_or_else(|| AssayError::WorktreeGitFailed { ... })` returning an actionable error
- Error message includes three remediation options: `git remote set-head origin --auto`, `init.defaultBranch` config, and explicit `base_branch` parameter

Updated the single callsite in `create()` (lines 272-275) from `unwrap_or_else` to `match` + `?` propagation.

### Task 2: Integration tests

Added two integration tests to the `integration_tests` module:

- `test_create_without_base_branch_no_remote_returns_error` — verifies that `create()` with `base_branch: None` in a repo without a remote returns an error mentioning all three remediation options
- `test_create_with_explicit_base_branch_skips_detection` — verifies that `create()` with `Some("main")` succeeds even without a remote, proving the bypass path

## Verification

- `cargo test --package assay-core -- worktree`: 24 passed (5 unit + 19 integration)
- `just test`: all tests pass across all workspace crates
- `just lint`: zero clippy warnings

## Commits

| Hash | Message |
|------|---------|
| efb832c | fix(46-02): make detect_default_branch() return Result instead of infallible String |
| 0cf7863 | test(46-02): add integration tests for fallible default branch detection |

## Deviations

None. Plan executed as written.
