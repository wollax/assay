---
estimated_steps: 5
estimated_files: 6
---

# T01: Add merge execution types and `merge_execute()` function

**Slice:** S03 — Sequential Merge Runner & Conflict Contract
**Milestone:** M002

## Description

Extend the existing `merge.rs` module with side-effecting merge execution. Today `merge_check()` detects conflicts read-only via `git merge-tree --write-tree`. This task adds `merge_execute()` that actually performs `git merge --no-ff <branch>`, returning a structured `MergeExecuteResult` with the merge commit SHA and changed files on success, or conflict details on failure (with automatic `git merge --abort`). Also adds conflict marker scanning utilities for post-merge validation.

All new types go in `assay-types/src/merge.rs` following existing patterns (`deny_unknown_fields`, schemars, inventory). The `merge_execute()` function goes in `assay-core/src/merge.rs`, reusing the existing `git_raw()`/`git_command()` helpers (changed to `pub(crate)` visibility).

## Steps

1. Add new types to `crates/assay-types/src/merge.rs`: `MergeExecuteResult` (merge_sha, files_changed, was_conflict, conflict_details), `ConflictScan` (has_markers, markers, truncated), `ConflictMarker` (file, line, marker_type). Add inventory registration for new types. Add `deny_unknown_fields` and schemars derives. Add schema snapshot tests in `crates/assay-types/tests/schema_snapshots.rs`.

2. Add `MergeExecuteError` variant to `AssayError` in `crates/assay-core/src/error.rs` — carry branch name, conflicting files, and message. Change `git_raw()` and `git_command()` in `merge.rs` from private to `pub(crate)`.

3. Implement `merge_execute(project_root, branch, message) -> Result<MergeExecuteResult>` in `crates/assay-core/src/merge.rs`: check for in-progress merge (`.git/MERGE_HEAD`), run `git merge --no-ff -m <message> <branch>`, on success extract merge SHA and diff-tree for files changed, on conflict run `git merge --abort` and return conflict details.

4. Implement `scan_conflict_markers(content) -> ConflictScan` (search for `<<<<<<<`, `=======`, `>>>>>>>` in a string) and `scan_files_for_markers(dir, files) -> ConflictScan` (scan multiple files, cap at 100 files with truncation flag).

5. Write integration tests using `tempfile` + `git init` pattern: clean merge succeeds with correct SHA and files, conflicting merge returns conflict details and leaves repo clean (abort worked), in-progress merge detected at startup, conflict marker scanning works on real files. Tests go in the existing `#[cfg(test)] mod tests` block in `merge.rs`.

## Must-Haves

- [ ] `MergeExecuteResult`, `ConflictScan`, `ConflictMarker` types in assay-types with full derives, `deny_unknown_fields`, inventory, schema snapshots
- [ ] `MergeExecuteError` variant on `AssayError`
- [ ] `git_raw()` and `git_command()` are `pub(crate)`
- [ ] `merge_execute()` performs `git merge --no-ff` and returns structured result
- [ ] `merge_execute()` runs `git merge --abort` on conflict and returns conflict details
- [ ] `merge_execute()` checks for in-progress merge state before starting
- [ ] `scan_conflict_markers()` and `scan_files_for_markers()` detect standard git conflict markers
- [ ] Integration tests with real git repos prove clean merge, conflict detection, and abort cleanup

## Verification

- `cargo test -p assay-types -- merge` — type round-trip and schema snapshot tests pass
- `cargo test -p assay-core -- merge::tests` — all merge tests pass (existing + new integration tests)
- `cargo clippy -p assay-core -p assay-types -- -D warnings` — no warnings

## Observability Impact

- Signals added/changed: `MergeExecuteResult` provides structured merge outcome — success with SHA and files, or conflict with file list and scan details
- How a future agent inspects this: deserialize `MergeExecuteResult` from merge runner report; check `was_conflict` flag and `conflict_details` for failure diagnosis
- Failure state exposed: `MergeExecuteError` carries branch name and conflicting files; `ConflictScan` provides line-level marker locations

## Inputs

- `crates/assay-core/src/merge.rs` — existing `merge_check()`, `git_raw()`, `git_command()` helpers, parser functions, test patterns
- `crates/assay-types/src/merge.rs` — existing `MergeCheck`, `MergeConflict`, `ConflictType`, `FileChange`, `ChangeType` types as pattern reference
- `crates/assay-core/src/error.rs` — existing `AssayError` enum with `MergeCheckRefError` as pattern reference

## Expected Output

- `crates/assay-types/src/merge.rs` — 3 new types (`MergeExecuteResult`, `ConflictScan`, `ConflictMarker`) with full derives and inventory
- `crates/assay-types/tests/schema_snapshots.rs` — new snapshot tests for merge execution types
- `crates/assay-types/tests/snapshots/` — new `.snap` files for merge execution type schemas
- `crates/assay-core/src/merge.rs` — `merge_execute()`, `scan_conflict_markers()`, `scan_files_for_markers()` functions + integration tests
- `crates/assay-core/src/error.rs` — `MergeExecuteError` variant
