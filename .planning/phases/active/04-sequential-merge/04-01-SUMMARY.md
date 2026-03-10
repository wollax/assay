# Phase 04 Plan 01: Git Primitives for Sequential Merge Summary

**One-liner:** SmeltError merge variants + 8 new GitOps/GitCli methods for squash merge, conflict detection, merge-base, branch creation, worktree checkout, reset, rev-parse, and diff-numstat.

## Frontmatter

- **Phase:** 04-sequential-merge
- **Plan:** 01
- **Subsystem:** git-operations
- **Tags:** git, merge, squash, conflict-detection, worktree
- **Completed:** 2026-03-10
- **Duration:** ~5 minutes

### Dependencies

- **Requires:** Phase 01-03 (GitOps trait, GitCli, worktree management)
- **Provides:** Merge git primitives (merge_base, merge_squash, branch_create, diff_numstat, unmerged_files, reset_hard, rev_parse, worktree_add_existing) + merge error variants
- **Affects:** 04-02 (MergeRunner), 04-03 (CLI merge command)

### Tech Stack

- **Added:** None (all git CLI shell-out via existing pattern)
- **Patterns:** Raw `tokio::process::Command` for exit code inspection in merge_squash (bypasses `run_in` error mapping)

### Key Files

- **Modified:**
  - `crates/smelt-core/src/error.rs` — 3 new SmeltError variants
  - `crates/smelt-core/src/git/mod.rs` — 8 new GitOps trait methods
  - `crates/smelt-core/src/git/cli.rs` — 8 GitCli implementations + 9 unit tests

### Decisions

| Decision | Rationale |
|----------|-----------|
| merge_squash checks both stdout and stderr for "CONFLICT" | git merge --squash writes conflict messages to stdout (not stderr), verified experimentally |
| merge_squash uses raw Command instead of run_in | Need to inspect exit codes — exit 0 = success, exit 1 + CONFLICT = merge conflict, other = git error |
| worktree_add_existing uses `git worktree add <path> <branch>` (no -b) | Checks out existing branch, unlike worktree_add which creates a new branch |
| reset_hard takes target_ref parameter | More flexible than always resetting to HEAD — needed for rollback scenarios |

## Tasks Completed

### Task 1: Add merge-specific error variants to SmeltError

- Added `MergeConflict { session, files }` — returned by merge_squash on conflict
- Added `MergeTargetExists { branch }` — for target branch collision
- Added `NoCompletedSessions` — for empty merge input
- **Commit:** 576df26

### Task 2: Extend GitOps trait and GitCli with merge-related methods

- 8 new trait methods: merge_base, branch_create, merge_squash, worktree_add_existing, unmerged_files, reset_hard, rev_parse, diff_numstat
- 8 GitCli implementations with appropriate git CLI shell-out
- 9 unit tests: test_merge_base, test_branch_create, test_merge_squash_clean, test_merge_squash_conflict, test_reset_hard, test_rev_parse, test_diff_numstat, test_unmerged_files_empty_when_clean, test_worktree_add_existing
- **Commit:** 8e00432

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed CONFLICT detection checking wrong output stream**

- **Found during:** Task 2 (test_merge_squash_conflict)
- **Issue:** Plan and research assumed `git merge --squash` writes "CONFLICT" to stderr. Experimentally verified it writes to stdout.
- **Fix:** merge_squash now checks both stdout and stderr for "CONFLICT" keyword
- **Files modified:** `crates/smelt-core/src/git/cli.rs`
- **Commit:** 8e00432

**2. [Rule 1 - Bug] Fixed test worktree path collision**

- **Found during:** Task 2 (test_merge_squash_conflict)
- **Issue:** Static worktree path `/tmp/smelt-test-squash-conflict` could collide between test runs
- **Fix:** Appended PID to worktree path for uniqueness
- **Files modified:** `crates/smelt-core/src/git/cli.rs`
- **Commit:** 8e00432

## Verification

- [x] `cargo build --workspace` compiles cleanly
- [x] `cargo test -p smelt-core` — 82 tests pass (73 existing + 9 new)
- [x] `cargo clippy --workspace -- -D warnings` — clean
- [x] SmeltError has MergeConflict, MergeTargetExists, NoCompletedSessions variants
- [x] GitOps trait has merge_base, branch_create, merge_squash, unmerged_files, reset_hard, rev_parse, diff_numstat, worktree_add_existing
- [x] GitCli implements all 8 new methods
- [x] Squash merge + conflict detection tested end-to-end

## Metrics

| Metric | Value |
|--------|-------|
| Tasks | 2/2 |
| Tests added | 9 |
| Tests total | 82 |
| Lines added (error.rs) | 12 |
| Lines added (mod.rs) | 52 |
| Lines added (cli.rs) | 453 |
| Artifact min_lines met | Yes (100/95, 176/135, 1118/280) |

## Next Phase Readiness

Plan 04-02 (MergeRunner) can proceed. All git primitives it depends on are implemented and tested:
- merge_base for finding common ancestor
- branch_create for target branch
- worktree_add_existing for temp merge worktree
- merge_squash for sequential squash merges with conflict detection
- reset_hard for rollback on failure
- diff_numstat for per-session merge stats
- rev_parse for commit hash resolution
