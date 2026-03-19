---
estimated_steps: 5
estimated_files: 5
---

# T01: Two-phase merge_execute and conflict resolution types

**Slice:** S01 — AI Conflict Resolution
**Milestone:** M003

## Description

Add an `abort_on_conflict: bool` parameter to `merge_execute()` that controls whether the function auto-aborts on conflict (existing behavior, default `true`) or leaves the working tree in a conflicted state for handler resolution (new behavior, `false`). Also create `ConflictResolutionConfig` type in assay-types with schema snapshot. This is the foundation for all subsequent tasks — without a live conflicted working tree, no handler can resolve anything (D044).

## Steps

1. Add `abort_on_conflict: bool` parameter to `merge_execute()` in `crates/assay-core/src/merge.rs`. In the conflict branch (exit code != 0), gate the `git merge --abort` block on `abort_on_conflict`. When `false`, skip abort and return `MergeExecuteResult` with `was_conflict: true` and `conflict_details` populated — the working tree remains conflicted.

2. Update all existing callers of `merge_execute()` to pass `abort_on_conflict: true`:
   - `crates/assay-core/src/orchestrate/merge_runner.rs` line ~141
   - Any direct calls in merge.rs tests

3. Add `ConflictResolutionConfig` to `crates/assay-types/src/orchestrate.rs`:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
   #[serde(deny_unknown_fields)]
   pub struct ConflictResolutionConfig {
       pub enabled: bool,
       #[serde(default = "default_conflict_model")]
       pub model: String,
       #[serde(default = "default_conflict_timeout")]
       pub timeout_secs: u64,
   }
   ```
   Add inventory schema submission. Add `Default` impl.

4. Add schema snapshot for `ConflictResolutionConfig` in `crates/assay-types/tests/schema_snapshots.rs`.

5. Add integration tests in `merge.rs` tests: create a real git repo with conflicting branches, call `merge_execute(..., false)`, verify the result has `was_conflict: true` AND the working tree still has conflict markers (read a conflicted file and assert it contains `<<<<<<<`). Then call `git merge --abort` manually to clean up. Also test that `merge_execute(..., true)` auto-aborts as before.

## Must-Haves

- [ ] `merge_execute()` accepts `abort_on_conflict: bool` parameter
- [ ] When `abort_on_conflict: false` and conflict occurs, working tree remains conflicted (not aborted)
- [ ] When `abort_on_conflict: true` (default), behavior is identical to current implementation
- [ ] All existing callers updated to pass `true`
- [ ] `ConflictResolutionConfig` type with serde/schemars derives, `deny_unknown_fields`, inventory submission
- [ ] Schema snapshot locked for `ConflictResolutionConfig`
- [ ] All existing tests pass unchanged

## Verification

- `cargo test -p assay-core merge` — existing merge tests pass + new two-phase tests
- `cargo test -p assay-types schema_snapshots` — new snapshot passes
- `cargo test -p assay-core merge_runner` — existing merge runner tests pass (they use `merge_execute` via the runner)

## Observability Impact

- Signals added/changed: `MergeExecuteResult` now distinguishes "conflict + aborted" from "conflict + live" via the caller's knowledge of which mode was used. No new fields needed — `was_conflict: true` with `conflict_details: Some(...)` is the same in both modes; the difference is whether the repo is clean after.
- How a future agent inspects this: Check `was_conflict` on the result. If the caller passed `abort_on_conflict: false`, they know the tree is still conflicted and must handle cleanup.
- Failure state exposed: If `abort_on_conflict: false` and the caller doesn't clean up, the repo stays in merge state — detectable via `MERGE_HEAD` existence (already checked at the top of `merge_execute`).

## Inputs

- `crates/assay-core/src/merge.rs` — current `merge_execute()` with auto-abort at lines 520-530
- `crates/assay-types/src/orchestrate.rs` — existing types to add `ConflictResolutionConfig` alongside
- D044 decision: two-phase merge lifecycle

## Expected Output

- `crates/assay-core/src/merge.rs` — `merge_execute()` with `abort_on_conflict` parameter + 2 new integration tests
- `crates/assay-types/src/orchestrate.rs` — `ConflictResolutionConfig` type with defaults and schema
- `crates/assay-types/tests/schema_snapshots.rs` — new snapshot entry
- All existing tests green
