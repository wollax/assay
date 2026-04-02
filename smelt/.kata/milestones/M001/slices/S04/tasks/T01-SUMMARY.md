---
id: T01
parent: S04
milestone: M001
provides:
  - ResultCollector<G: GitOps> with collect() method
  - BranchCollectResult struct for collection outcomes
  - 5 unit tests covering all edge cases
key_files:
  - crates/smelt-core/src/collector.rs
  - crates/smelt-core/src/lib.rs
key_decisions: []
patterns_established:
  - ResultCollector uses generics (not dyn) for GitOps due to RPITIT
  - Test helper pattern: setup_test_repo() + add_commit() + head_hash() for collector tests
observability_surfaces:
  - "tracing::info for HEAD/base_ref resolution, commit count, target branch creation"
  - "tracing::warn for no-changes, dirty worktree, target branch overwrite"
duration: 5m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Create ResultCollector with unit tests

**Created `ResultCollector<G: GitOps>` with `collect()` that reads git state and creates target branches, plus 5 unit tests covering all edge cases.**

## What Happened

Created `crates/smelt-core/src/collector.rs` with:
- `BranchCollectResult` struct with fields: branch, commit_count, files_changed, subjects, no_changes
- `ResultCollector<G: GitOps>` generic struct holding a `G` instance and repo path
- `collect(base_ref, target_branch)` method that resolves HEAD, compares to base_ref, and creates/updates the target branch

The `collect()` flow: rev_parse HEAD and base_ref → early return with `no_changes: true` if equal → check dirty worktree (warn) → count commits → get changed files and subjects → handle pre-existing target branch (delete + recreate) → create branch → return result.

Registered `pub mod collector` in `lib.rs` with re-exports for `ResultCollector` and `BranchCollectResult`.

## Verification

- `cargo test -p smelt-core -- collector::tests` — **5/5 passed**
  - `test_collect_basic` — 1 commit after base, branch created, files/subjects correct
  - `test_collect_no_changes` — HEAD == base, no_changes=true, no branch created
  - `test_collect_target_already_exists` — pre-existing branch force-updated to new HEAD
  - `test_collect_multiple_commits` — 3 commits, count=3, all files and subjects collected
  - `test_collect_dirty_worktree` — uncommitted changes present, collects committed changes only
- `cargo test --workspace` — **94 passed, 0 failed**, zero regressions

Slice-level verification (intermediate task — partial expected):
- ✅ `cargo test -p smelt-core -- collector::tests` — passes
- ⏳ `cargo test -p smelt-cli --test docker_lifecycle -- collect` — T02 scope
- ✅ `cargo test --workspace` — passes

## Diagnostics

- `SMELT_LOG=info cargo test -p smelt-core -- collector` shows collector decision points (HEAD/base resolution, commit count, branch creation)
- `tracing::warn` emitted for: no new commits, dirty working tree, target branch already exists (with old/new hashes)
- Errors surface as `SmeltError::GitExecution` for git command failures

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/collector.rs` — new: BranchCollectResult, ResultCollector<G: GitOps>, collect(), 5 unit tests
- `crates/smelt-core/src/lib.rs` — added `pub mod collector` and re-exports
