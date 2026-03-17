---
id: T01
parent: S03
milestone: M002
provides:
  - merge_execute() function for side-effecting git merge --no-ff
  - scan_conflict_markers() and scan_files_for_markers() for conflict marker detection
  - MergeExecuteResult, ConflictScan, ConflictMarker types in assay-types
  - MergeExecuteError variant on AssayError
  - pub(crate) visibility on git_raw() and git_command()
key_files:
  - crates/assay-types/src/merge.rs
  - crates/assay-core/src/merge.rs
  - crates/assay-core/src/error.rs
  - crates/assay-types/tests/schema_snapshots.rs
key_decisions:
  - ConflictScan used as the shared type for both merge_execute conflict details and standalone marker scanning
  - scan_conflict_markers takes a file_path argument rather than inferring it, so callers control the path representation
  - Worktree .git file handling in merge_execute for MERGE_HEAD detection (reads gitdir path from .git file)
patterns_established:
  - Integration test helper setup_git_repo() creates temp git repos with initial commit for merge testing
  - merge_execute returns Ok(MergeExecuteResult) for both success and conflict cases; errors reserved for infrastructure failures
observability_surfaces:
  - MergeExecuteResult.was_conflict flag for programmatic conflict detection
  - MergeExecuteResult.conflict_details provides ConflictScan with line-level marker locations
  - MergeExecuteError carries branch name and conflicting file list for diagnostics
duration: 20m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add merge execution types and `merge_execute()` function

**Added `merge_execute()` for side-effecting `git merge --no-ff` with structured conflict detection and automatic abort cleanup.**

## What Happened

Extended `assay-types/src/merge.rs` with three new types: `MergeExecuteResult` (merge outcome with SHA, changed files, or conflict details), `ConflictScan` (marker scan results), and `ConflictMarker` (individual marker with file/line/type). Added `MarkerType` enum for the three standard git conflict markers. All types have `deny_unknown_fields`, schemars derives, and inventory registration.

Added `MergeExecuteError` variant to `AssayError` in `assay-core/src/error.rs` carrying branch name, conflicting files, and message.

Changed `git_raw()` and `git_command()` from private to `pub(crate)` in `merge.rs`.

Implemented `merge_execute()` which: checks for in-progress merge (MERGE_HEAD), runs `git merge --no-ff -m <message> <branch>`, on success extracts merge SHA via `rev-parse HEAD` and changed files via `diff-tree`, on conflict collects conflicting files via `git diff --name-only --diff-filter=U`, scans for conflict markers, runs `git merge --abort`, and returns structured result.

Implemented `scan_conflict_markers()` (scans string content for `<<<<<<<`, `=======`, `>>>>>>>` markers) and `scan_files_for_markers()` (scans multiple files with 100-file cap and truncation flag).

Wrote integration tests using real git repos (tempfile + git init): clean merge succeeds with correct SHA and files, conflicting merge returns conflict details and leaves repo clean after abort, in-progress merge detected at startup, and conflict marker scanning unit tests.

## Verification

- `cargo test -p assay-types --test schema_snapshots` — 38 passed (3 new snapshot tests accepted)
- `cargo test -p assay-core -- merge::tests` — 27 passed (7 new tests: 4 marker scanning + 3 integration)
- `cargo test -p assay-core --features orchestrate -- merge::tests` — 27 passed
- `cargo clippy -p assay-core -p assay-types -- -D warnings` — clean, no warnings

### Slice-level verification (partial — T01 of 3):
- ✅ `cargo test -p assay-core --features orchestrate -- merge::tests` — passes
- ⏳ `cargo test -p assay-core --features orchestrate -- orchestrate::ordering` — not yet (T02)
- ⏳ `cargo test -p assay-core --features orchestrate -- orchestrate::merge_runner` — not yet (T03)
- ⏳ `cargo test -p assay-types --features orchestrate -- merge_runner` — not yet (T02/T03)
- ⏳ `just ready` — deferred to T03

## Diagnostics

- Deserialize `MergeExecuteResult` and check `was_conflict` flag for merge outcome
- On conflict: `conflict_details.markers` provides file/line/marker_type for each conflict marker found
- `MergeExecuteError` in error chain provides branch name and conflicting file list
- `ConflictScan.truncated` indicates if file scan was capped at 100 files

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/merge.rs` — added `MarkerType`, `ConflictMarker`, `ConflictScan`, `MergeExecuteResult` types with inventory registration
- `crates/assay-types/src/lib.rs` — added new type exports
- `crates/assay-types/tests/schema_snapshots.rs` — added 3 schema snapshot tests
- `crates/assay-types/tests/snapshots/merge-execute-result-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/conflict-scan-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/conflict-marker-schema.snap` — new snapshot
- `crates/assay-core/src/error.rs` — added `MergeExecuteError` variant
- `crates/assay-core/src/merge.rs` — changed `git_raw()`/`git_command()` to `pub(crate)`, added `merge_execute()`, `scan_conflict_markers()`, `scan_files_for_markers()`, and 7 new tests
