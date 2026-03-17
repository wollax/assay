---
estimated_steps: 5
estimated_files: 3
---

# T02: Orphan detection and collision prevention

**Slice:** S05 — Worktree Enhancements & Tech Debt
**Milestone:** M001

## Description

Implement `detect_orphans()` to find worktrees with no active work session, and add collision prevention to `create()` that rejects worktree creation when the spec already has an active worktree with an in-progress session. Both use the session linkage from T01.

## Steps

1. Add `WorktreeCollision` error variant to `assay-core/src/error.rs` with `spec_slug: String` and `existing_path: PathBuf` fields. Include actionable message mentioning cleanup.
2. Implement `detect_orphans(project_root: &Path, assay_dir: &Path) -> Result<Vec<WorktreeInfo>>` in `assay-core/src/worktree.rs`. Logic: call `list(project_root)` to get all worktrees, for each entry read metadata to get `session_id`. A worktree is orphaned if: (a) `session_id` is `None`, or (b) session_id points to a session that doesn't exist on disk, or (c) session_id points to a session whose phase is terminal (`Completed` or `Abandoned`). Use `work_session::load_session()` and `SessionPhase::is_terminal()`.
3. Add collision check at the top of `create()` — before the filesystem/branch existence checks. Call `list(project_root)` and check for any existing worktree with matching `spec_slug`. For each match, read metadata and load the linked session. Reject with `WorktreeCollision` if the session is active (non-terminal). Allow creation if the matching worktree has no session or a terminal session (orphan scenario — user should clean up but not blocked).
4. Write unit tests for `detect_orphans`: (a) worktree with no session_id is orphaned, (b) worktree with active session is NOT orphaned, (c) worktree with terminal session IS orphaned, (d) worktree with missing session record IS orphaned. These will need mock filesystem setup — use the existing temp-dir integration test pattern from the worktree module.
5. Write unit tests for collision prevention: (a) `create()` rejects with `WorktreeCollision` when spec has active worktree, (b) `create()` succeeds when spec has worktree with terminal session, (c) `create()` succeeds when no worktree exists for spec. Note: collision tests require real git repos — follow the existing `#[cfg(test)] mod integration_tests` pattern.

## Must-Haves

- [ ] `WorktreeCollision` error variant with spec_slug and existing_path
- [ ] `detect_orphans()` correctly identifies orphaned worktrees
- [ ] `create()` rejects when spec has active worktree with in-progress session
- [ ] `create()` allows when matching worktree has terminal/missing session
- [ ] Tests for orphan detection (4 scenarios)
- [ ] Tests for collision prevention (3 scenarios)

## Verification

- `cargo test -p assay-core -- worktree` — all tests pass including new orphan and collision tests
- `cargo build --workspace` — clean compilation (error variant used correctly)

## Observability Impact

- Signals added/changed: `WorktreeCollision` error includes spec_slug and existing worktree path for actionable diagnosis
- How a future agent inspects this: call `detect_orphans()` to see all orphaned worktrees; `WorktreeCollision` error message guides cleanup
- Failure state exposed: collision error includes the path of the conflicting worktree and the spec_slug

## Inputs

- `crates/assay-core/src/worktree.rs` — `create()` with session_id param from T01, `list()`, `read_metadata()`
- `crates/assay-core/src/work_session.rs` — `list_sessions()`, `load_session()`
- `crates/assay-types/src/work_session.rs` — `SessionPhase::is_terminal()`
- T01 output: `WorktreeMetadata.session_id` field, updated `create()` signature

## Expected Output

- `crates/assay-core/src/error.rs` — `WorktreeCollision` variant added
- `crates/assay-core/src/worktree.rs` — `detect_orphans()` function + collision check in `create()` + 7 new tests
