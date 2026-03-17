---
id: T02
parent: S05
milestone: M001
provides:
  - detect_orphans() function identifying worktrees with no active session
  - WorktreeCollision error variant for active-session collision prevention
  - collision check in create() rejecting duplicate active worktrees per spec
key_files:
  - crates/assay-core/src/error.rs
  - crates/assay-core/src/worktree.rs
key_decisions:
  - Collision check derives assay_dir from specs_dir parent rather than adding a new parameter to create()
  - Collision check uses Option::and_then chaining to satisfy clippy's collapsed-if requirement
patterns_established:
  - detect_orphans uses list() + read_metadata + load_session composition pattern
  - Collision check runs before filesystem/branch checks for clearer error messages
observability_surfaces:
  - WorktreeCollision error includes spec_slug and existing_path for actionable diagnosis
  - detect_orphans() returns Vec<WorktreeInfo> listing all orphaned worktrees with paths
duration: 12m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T02: Orphan detection and collision prevention

**Added `detect_orphans()` to find worktrees with no active work session, and collision prevention in `create()` that rejects worktree creation when the spec already has an active worktree.**

## What Happened

1. Added `WorktreeCollision` error variant to `AssayError` with `spec_slug` and `existing_path` fields, including an actionable message guiding users to complete/abandon the session and clean up.

2. Implemented `detect_orphans(project_root, assay_dir)` that calls `list()`, reads metadata for each worktree, and classifies as orphaned if: no session_id, session doesn't exist on disk, or session is in a terminal phase (Completed/Abandoned).

3. Added collision check at the top of `create()` — before filesystem/branch checks. Uses `list()` to find existing worktrees for the same spec, reads metadata, loads linked session, and rejects with `WorktreeCollision` if session is active (non-terminal). Derives `assay_dir` from `specs_dir.parent()` to avoid changing the `create()` signature.

4. Added 4 orphan detection tests: no session_id → orphaned, active session → not orphaned, terminal session → orphaned, missing session → orphaned.

5. Added 3 collision prevention tests: active session → WorktreeCollision, terminal session → allowed (falls through to WorktreeExists), no existing worktree → succeeds.

## Verification

- `cargo test -p assay-core -- worktree` — 32 tests pass (7 new: 4 orphan + 3 collision)
- `cargo build --workspace` — clean compilation
- `just ready` — all checks pass (fmt, lint, test, deny)

## Diagnostics

- Call `detect_orphans(project_root, assay_dir)` to list all orphaned worktrees with their paths
- `WorktreeCollision` error message includes the spec slug and existing worktree path for targeted cleanup
- Read `.assay/worktree.json` in any worktree to inspect `session_id` linkage

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/error.rs` — Added `WorktreeCollision` error variant
- `crates/assay-core/src/worktree.rs` — Added `detect_orphans()`, collision check in `create()`, 7 new tests
