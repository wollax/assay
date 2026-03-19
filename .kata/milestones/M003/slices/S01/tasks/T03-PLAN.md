---
estimated_steps: 4
estimated_files: 3
---

# T03: Wire conflict handler into merge runner lifecycle

**Slice:** S01 — AI Conflict Resolution
**Milestone:** M003

## Description

Connect T01's two-phase `merge_execute()` and T02's `resolve_conflict()` into the merge runner loop. When `conflict_resolution_enabled` is true on `MergeRunnerConfig`, the merge runner calls `merge_execute(..., abort_on_conflict: false)`, invokes the handler with the live conflicted tree, and manages the lifecycle: verify commit on success, `git merge --abort` on failure or panic. Prove with an integration test using a real git repo and a scripted resolver (shell-like closure that strips markers, stages, and commits).

## Steps

1. Add `conflict_resolution_enabled: bool` field to `MergeRunnerConfig` (default `false`). This controls whether the merge runner uses two-phase merge or the existing auto-abort path.

2. Modify the merge loop in `merge_completed_sessions()`:
   - When `conflict_resolution_enabled` is true, call `merge_execute(project_root, branch, message, false)` (don't abort on conflict).
   - If the result is a conflict, wrap the handler invocation in `std::panic::catch_unwind()`.
   - If handler returns `Resolved(sha)`, verify the commit exists with `git rev-parse --verify <sha>`. If valid, record as `Merged`.
   - If handler returns `Skip` or `Abort`, or if handler panics, run `git merge --abort` to clean up the conflicted tree, then record accordingly.
   - When `conflict_resolution_enabled` is false (default), existing behavior: `merge_execute(..., true)` + handler receives post-abort scan. No changes to this path.

3. Add integration test `merge_runner_conflict_resolution_with_live_tree`: create a real git repo with two branches that modify the same file differently (actual conflict). Use a scripted handler closure that: reads the conflicted file, strips all conflict markers keeping both sides, writes the resolved content, runs `git add`, runs `git commit -m "resolved"`, returns `ConflictAction::Resolved(sha)`. Assert: `MergeReport` shows 1 `Merged`, 0 `ConflictSkipped`. Verify the final repo has the merge commit with both parents.

4. Add integration test `merge_runner_conflict_resolution_handler_failure`: same repo setup, but handler returns `ConflictAction::Skip`. Assert: `MergeReport` shows 1 `ConflictSkipped`. Verify repo is clean (no MERGE_HEAD). Also test handler panic path — assert repo is still clean after panic recovery.

## Must-Haves

- [ ] `MergeRunnerConfig.conflict_resolution_enabled` field (default `false`)
- [ ] Two-phase merge path: `merge_execute(..., false)` when resolution enabled
- [ ] Handler panic safety: `catch_unwind` + `git merge --abort` on panic
- [ ] Commit SHA verification via `git rev-parse --verify`
- [ ] Cleanup: `git merge --abort` when handler fails or skips
- [ ] Integration test proving real conflict → live handler → resolved merge → valid report
- [ ] Integration test proving handler failure → clean abort → no dirty repo
- [ ] All existing merge runner tests pass unchanged (they use default `conflict_resolution_enabled: false`)

## Verification

- `cargo test -p assay-core merge_runner` — all existing + 2 new integration tests pass
- `cargo test -p assay-core merge` — existing merge tests still pass
- Manually verify: the new integration test creates a real merge commit with `git log --oneline --graph` equivalent assertions

## Observability Impact

- Signals added/changed: The merge runner now distinguishes "conflict skipped (default handler)" from "conflict resolved (AI handler)" in its per-session results. Both use existing `MergeSessionStatus` variants (`Merged` vs `ConflictSkipped`).
- How a future agent inspects this: `MergeReport.results` per-session entries show the outcome. A resolved conflict shows `Merged` with a valid `merge_sha`. A failed resolution shows `ConflictSkipped` with an `error` message describing why.
- Failure state exposed: Handler panics are caught and logged. The error message includes "conflict handler panicked" text. The repo is always left clean (abort on failure).

## Inputs

- `crates/assay-core/src/merge.rs` — `merge_execute()` with `abort_on_conflict` parameter (T01)
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — `resolve_conflict()` function (T02)
- `crates/assay-core/src/orchestrate/merge_runner.rs` — existing merge loop and `MergeRunnerConfig`
- Existing test helpers: `setup_git_repo()`, `create_branch_with_file()` in merge_runner tests

## Expected Output

- `crates/assay-core/src/orchestrate/merge_runner.rs` — updated merge loop with two-phase path + `conflict_resolution_enabled` config + 2 new integration tests
- Proof: integration test demonstrates the full conflict resolution lifecycle with a real git repo
