---
phase: 47-merge-check
plan: 01
subsystem: core/merge
tags: [git, merge-tree, conflict-detection, types]
requires: []
provides:
  - MergeCheck type in assay-types
  - merge_check() function in assay-core
  - MergeCheckRefError and GitVersionTooOld error variants
affects:
  - Phase 47 Plan 02 (MCP tool wiring)
  - Phase 50 (merge_propose)
tech-stack:
  added: []
  patterns:
    - git merge-tree --write-tree for zero side-effect conflict detection
    - git_raw() helper for inspecting exit codes directly
key-files:
  created:
    - crates/assay-types/src/merge.rs
    - crates/assay-core/src/merge.rs
  modified:
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/lib.rs
    - crates/assay-core/src/error.rs
decisions:
  - Duplicated git_command helper in merge module (same pattern as worktree.rs) rather than extracting shared module — refactor deferred
  - Added git_raw() variant that returns (stdout, stderr, exit_code) for merge-tree exit code inspection
  - parse_conflicts extracts paths from informational messages using pattern matching on known message formats
metrics:
  duration: 5m22s
  completed: 2026-03-16
---

# Phase 47 Plan 01: Merge Check Types & Core Logic Summary

**Merge check foundation using `git merge-tree --write-tree` for zero side-effect conflict detection with full type definitions and parsing helpers.**

## What Was Done

### Task 1: Merge Check Types (assay-types)
Created `crates/assay-types/src/merge.rs` with five types:
- `ChangeType` enum (Added, Modified, Deleted) with Display impl
- `FileChange` struct (path + change_type)
- `ConflictType` enum (Content, RenameDelete, RenameRename, ModifyDelete, AddAdd, FileDirectory, Binary, Submodule, Other) with Display impl
- `MergeConflict` struct (path + conflict_type + message)
- `MergeCheck` struct (clean, base_sha, head_sha, merge_base_sha, fast_forward, ahead, behind, files, conflicts, truncated)

All types have serde + schemars derives. MergeCheck registered in schema registry. Re-exports wired in lib.rs.

### Task 2: Error Variants
Added two variants to `AssayError`:
- `MergeCheckRefError { message }` — actionable message when refs fail to resolve
- `GitVersionTooOld { version }` — when git < 2.38 detected

### Task 3: Core Logic (assay-core)
Created `crates/assay-core/src/merge.rs` with `merge_check()` function orchestrating 6 git commands:
1. `git rev-parse` for both refs (collects both errors if both fail)
2. `git merge-base` for common ancestor (graceful failure for unrelated histories)
3. `git merge-base --is-ancestor` for fast-forward detection
4. `git rev-list --left-right --count` for ahead/behind
5. `git merge-tree --write-tree` for conflict detection
6. `git diff-tree -r --name-status` for clean merge file list

All six documented pitfalls (P1-P6) handled:
- P1: Exit code 1 disambiguated by checking stdout for valid 40-char hex OID
- P2: Clean merges get file list via diff-tree follow-up
- P3: Both "content" and "contents" map to ConflictType::Content
- P4: Exit 128 returns error, not conflicts
- P5: git rev-parse without --verify for relative ref support
- P6: merge-base failure produces merge_base_sha: None

14 unit tests covering all parsing helpers.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `just test` — all workspace tests pass (573+ tests)
- `just lint` — zero clippy warnings
