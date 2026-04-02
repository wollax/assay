# Phase 04 Plan 02: MergeRunner — Core Merge Engine Summary

**One-liner:** MergeRunner orchestrates full sequential squash-merge pipeline — session filtering, temp worktree, atomic rollback on conflict, cleanup on success, and MergeReport with per-session diff stats.

## Frontmatter

- **Phase:** 04-sequential-merge
- **Plan:** 02
- **Subsystem:** merge-engine
- **Tags:** merge, squash, rollback, worktree, pipeline
- **Completed:** 2026-03-10
- **Duration:** ~5 minutes

### Dependencies

- **Requires:** 04-01 (git merge primitives: merge_squash, branch_create, worktree_add_existing, reset_hard, diff_numstat, rev_parse)
- **Provides:** MergeRunner with run() method, MergeOpts, MergeReport, MergeSessionResult, DiffStat, full merge pipeline
- **Affects:** 04-03 (CLI merge command)

### Tech Stack

- **Added:** None (all existing crate dependencies)
- **Patterns:** MergeRunner<G: GitOps + Clone> follows SessionRunner pattern; explicit cleanup in error paths (no Drop guard)

### Key Files

- **Created:**
  - `crates/smelt-core/src/merge/types.rs` — MergeOpts, MergeReport, MergeSessionResult, DiffStat (46 lines)
  - `crates/smelt-core/src/merge/mod.rs` — MergeRunner, format_commit_message, 7 integration tests (592 lines)
- **Modified:**
  - `crates/smelt-core/src/lib.rs` — `pub mod merge;` + re-exports (15 lines)

### Decisions

| Decision | Rationale |
|----------|-----------|
| Explicit cleanup in error paths (not Drop guard) | Simpler, consistent with plan recommendation, all paths are explicit |
| Template commit messages (no LLM) | Works out of the box, LLM enhancement deferred per research doc |
| diff_numstat with `{hash}^` parent ref | Simpler than tracking pre-merge HEAD; git resolves short hashes correctly |
| WorktreeManager::remove for session cleanup | Reuses existing remove(force=true) which handles worktree + branch + state file |
| MergeReport::has_skipped() convenience method | Useful for CLI display logic in 04-03 |

## Tasks Completed

### Task 1: Define merge types

- Created `types.rs` with MergeOpts (derive Default), MergeReport, MergeSessionResult, DiffStat
- Created `mod.rs` with module declarations and re-exports
- Updated `lib.rs` with `pub mod merge` and `pub use merge::{MergeOpts, MergeReport}`
- **Commit:** f512aeb

### Task 2: Implement MergeRunner with full merge pipeline

- MergeRunner<G: GitOps + Clone> with new() and run() methods
- Phase A (Preparation): validates .smelt/, reads session state files, checks for Running sessions, filters to Completed, errors on zero completed
- Phase B (Target branch): determines branch name (default or custom), checks existence, resolves base commit via rev_parse, creates target branch
- Phase C (Temp worktree): computes sibling path, checks out target branch via worktree_add_existing
- Phase D (Sequential merge): iterates completed sessions in manifest order, squash-merges each, commits with template messages, collects diff stats
- Phase E (Cleanup): success path removes temp worktree + prunes + removes session worktrees/branches via WorktreeManager; error path resets, removes temp worktree, deletes target branch
- format_commit_message(): template with 72-char subject truncation, body with session metadata
- 7 integration tests using real GitCli + temp repos
- **Commits:** 9e569a9, 0044dc3

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- [x] `cargo build --workspace` compiles cleanly
- [x] `cargo test -p smelt-core` — 89 tests pass (82 existing + 7 new)
- [x] `cargo clippy --workspace -- -D warnings` — clean
- [x] Two clean sessions merge into target branch with combined changes
- [x] Conflict triggers full rollback (target branch deleted, temp worktree removed)
- [x] Failed sessions skipped, Running sessions blocked
- [x] MergeReport contains per-session diff stats
- [x] Target branch name collision detected
- [x] NoCompletedSessions error when all sessions failed
- [x] Custom target branch name works
- [x] Temp worktree always cleaned up (success and failure paths)
- [x] Session worktrees cleaned up after successful merge

## Metrics

| Metric | Value |
|--------|-------|
| Tasks | 2/2 |
| Tests added | 7 |
| Tests total | 89 |
| Lines added (mod.rs) | 592 |
| Lines added (types.rs) | 46 |
| Lines added (lib.rs) | 2 |
| Artifact min_lines met | Yes (592/200, 46/40, 15/15) |

## Next Phase Readiness

Plan 04-03 (CLI merge command) can proceed. MergeRunner is fully functional with:
- run() accepts Manifest + MergeOpts, returns MergeReport
- All error cases handled with appropriate SmeltError variants
- MergeReport provides all data needed for CLI output formatting
