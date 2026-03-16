---
phase: 46-worktree-fixes
plan: 01
subsystem: worktree
tags: [canonicalization, path-resolution, symlinks, macos]
depends_on: []
files_modified:
  - crates/assay-core/src/worktree.rs
decisions:
  - "Best-effort canonicalization at end of resolve_worktree_dir() — single fix point for all consumers"
  - "Three-branch fallback: full path exists -> canonicalize; parent exists -> canonicalize parent + leaf; neither -> return as-is"
  - "Existing unit tests use fake paths and fall through to as-is fallback — no assertion changes needed"
metrics:
  tasks_completed: 2
  tasks_total: 2
  deviations: 0
  duration: "~3 min"
---

# Phase 46 Plan 01: Path Canonicalization Summary

## Objective

Add best-effort path canonicalization to `resolve_worktree_dir()` so worktree base directory paths resolve symlinks and `..` segments, matching the canonical paths returned by `git worktree list --porcelain`.

## What Changed

### Task 1: Canonicalization logic in resolve_worktree_dir()

Added a three-branch canonicalization block at the end of `resolve_worktree_dir()` (lines 207-219 of `crates/assay-core/src/worktree.rs`):

1. **Full path exists** — `std::fs::canonicalize(&resolved)` resolves symlinks and collapses `..`
2. **Parent exists, leaf doesn't** — canonicalize the parent, append the leaf (worktree dir not yet created)
3. **Neither exists** — return path as-is (fallback for fake/test paths)

All 5 existing unit tests pass unchanged (they use fake paths like `/home/user/myproject` which hit the fallback).

### Task 2: Integration tests

Added two integration tests to the `integration_tests` module:

- `test_resolve_worktree_dir_canonicalizes_dotdot_segments` — creates a real TempDir, uses a relative `../myproject-worktrees` config path, verifies no `..` segments remain in the result
- `test_resolve_worktree_dir_canonicalizes_symlinks` — creates a symlink to a real directory, passes it as the worktree base dir, verifies the result points to the real path

Both tests canonicalize the TempDir path itself before comparison to handle the macOS `/var/folders` -> `/private/var/folders` symlink.

## Verification

- `cargo test --package assay-core -- worktree`: 22 passed (5 unit + 17 integration)
- `just test`: 838 passed across all workspace crates
- `just lint`: zero clippy warnings

## Commits

| Hash | Message |
|------|---------|
| a4b47e2 | feat(46-01): add best-effort path canonicalization to resolve_worktree_dir() |
| a9eb3c5 | test(46-01): add integration tests for path canonicalization |

## Deviations

None. Plan executed as written.
