---
id: T03
parent: S01
milestone: M003
provides:
  - Two-phase merge path in merge_completed_sessions() with conflict_resolution_enabled config
  - Handler panic safety via catch_unwind + git merge --abort cleanup
  - Commit SHA verification via git rev-parse --verify
  - Integration tests proving live conflict resolution and handler failure/panic recovery
key_files:
  - crates/assay-core/src/orchestrate/merge_runner.rs
key_decisions:
  - Handler panic recovery uses catch_unwind(AssertUnwindSafe) wrapping the conflict handler closure; panics result in ConflictSkipped status with descriptive error
  - SHA verification uses git_raw (not git_command) to check exit code without propagating errors
  - Invalid SHA from handler triggers merge abort + ConflictSkipped (not Failed) to allow the loop to continue
patterns_established:
  - Two-phase merge lifecycle integration: conflict_resolution_enabled toggles between abort_on_conflict=true (default) and abort_on_conflict=false (live tree) paths
  - Panic-safe handler invocation pattern with automatic git merge --abort cleanup
observability_surfaces:
  - MergeSessionResult.error contains "conflict handler panicked" on panic, "conflict handler returned invalid SHA" on bad SHA, "conflict skipped" on skip
  - MergeReport.results distinguishes Merged (resolved) from ConflictSkipped (failed resolution) with descriptive errors
duration: ~15min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Wire conflict handler into merge runner lifecycle

**Connected two-phase merge_execute() and conflict handler into merge_completed_sessions() with panic safety, SHA verification, and automatic abort cleanup — proven by integration tests with real git repos.**

## What Happened

Added `conflict_resolution_enabled: bool` field to `MergeRunnerConfig` (default `false`). When enabled, the merge loop calls `merge_execute(..., abort_on_conflict: false)` leaving the working tree conflicted so the handler can resolve in-place. The handler invocation is wrapped in `std::panic::catch_unwind()` for safety. On success, the returned SHA is verified with `git rev-parse --verify`. On failure (Skip, Abort, panic, invalid SHA), `git merge --abort` cleans up the conflicted tree.

All existing code paths (CLI, MCP, integration tests) were updated to pass `conflict_resolution_enabled: false`, preserving existing auto-abort behavior.

Two integration tests prove the lifecycle:
1. `test_merge_runner_conflict_resolution_with_live_tree`: real git repo with conflicting branches → scripted handler strips markers, stages, commits → MergeReport shows Merged with valid merge commit (2 parents verified)
2. `test_merge_runner_conflict_resolution_handler_failure`: handler returns Skip → merge aborted, repo clean; handler panics → panic caught, merge aborted, repo clean

## Verification

- `cargo test -p assay-core --features orchestrate "orchestrate::merge_runner"` — 8 tests pass (6 existing + 2 new)
- `cargo test -p assay-core --features orchestrate "merge"` — 39 merge tests + 1 orchestrate integration test pass
- `cargo test -p assay-core --features orchestrate "resolve_conflict"` — T02 tests still pass
- `cargo build` — full workspace builds clean (only pre-existing warning about unused git_raw import in conflict_resolver.rs)

### Slice-level verification status (intermediate task — partial passes expected):
- ✅ `cargo test -p assay-core merge_execute_two_phase` — pass
- ✅ `cargo test -p assay-core merge_runner_conflict_resolution` — pass (2 new tests)
- ✅ `cargo test -p assay-core resolve_conflict` — pass
- ⬜ `cargo test -p assay-types schema_snapshots` — no new schemas this task (pass, nothing to test)
- ⬜ `cargo test -p assay-cli run` — CLI flag not yet wired (T04/T05)
- ⬜ `cargo test -p assay-mcp orchestrate_run` — MCP parameter not yet wired (T04/T05)
- ⬜ `just ready` — deferred to final task

## Diagnostics

- `MergeSessionResult.error` field carries descriptive failure reasons: "conflict handler panicked — merge aborted", "conflict handler returned invalid SHA: <sha>", "conflict skipped — conflicting files: <list>"
- `MergeReport.results` per-session entries show Merged (with valid merge_sha) for resolved conflicts, ConflictSkipped for failed resolutions
- Repo state is always clean after the merge loop completes (no dangling MERGE_HEAD)

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/merge_runner.rs` — Added `conflict_resolution_enabled` to config, two-phase merge path with catch_unwind + SHA verification + abort cleanup, 2 integration tests
- `crates/assay-cli/src/commands/run.rs` — Added `conflict_resolution_enabled: false` to existing config construction
- `crates/assay-mcp/src/server.rs` — Added `conflict_resolution_enabled: false` to existing config construction
- `crates/assay-core/tests/orchestrate_integration.rs` — Added `conflict_resolution_enabled: false` to existing config constructions
