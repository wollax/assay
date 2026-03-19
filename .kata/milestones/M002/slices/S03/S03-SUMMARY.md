---
id: S03
parent: M002
milestone: M002
provides:
  - merge_execute() function for side-effecting `git merge --no-ff` with structured conflict detection
  - scan_conflict_markers() and scan_files_for_markers() for conflict marker detection
  - order_sessions() with CompletionTime and FileOverlap merge ordering strategies
  - merge_completed_sessions() sequencing loop with closure-based conflict handler
  - default_conflict_handler() returning ConflictAction::Skip
  - extract_completed_sessions() bridging OrchestratorResult to CompletedSession
  - MergeExecuteResult, ConflictScan, ConflictMarker, MergeReport, MergePlan, ConflictAction types
  - MergeExecuteError and MergeRunnerError variants on AssayError
requires:
  - slice: S01
    provides: DependencyGraph::topological_groups() for merge ordering
  - slice: S02
    provides: SessionOutcome::Completed with branch_name, changed_files; OrchestratorResult
affects:
  - S06
key_files:
  - crates/assay-types/src/merge.rs
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-core/src/merge.rs
  - crates/assay-core/src/orchestrate/ordering.rs
  - crates/assay-core/src/orchestrate/merge_runner.rs
  - crates/assay-core/src/error.rs
key_decisions:
  - D019 (merge execution strategy) — topological-order sequential merge with re-check before each merge
  - D025 (merge ordering strategies) — CompletionTime + FileOverlap strategies ported from Smelt
  - D026 (conflict handler contract) — closure-based Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction
  - CompletedSession is operational (not serializable) — lives in assay-core, not assay-types
  - Three-level deterministic tiebreaking for both strategies (timestamp/overlap, topo_order, session_name)
  - MergeRunnerConfig pre-flight validates clean working tree and no in-progress merge before any merge attempt
patterns_established:
  - Integration tests use setup_git_repo() helper creating temp repos with initial commit
  - merge_execute() returns Ok for both success and conflict cases; errors reserved for infrastructure failures
  - Ordering strategies as pure functions returning (Vec<CompletedSession>, MergePlan) tuple
  - MergePlan entries carry human-readable reason strings for per-session placement rationale
  - Pre-flight validation (clean tree + no MERGE_HEAD) before merge loop
  - Abort flag propagation through remaining sessions
observability_surfaces:
  - MergeReport provides per-session merge status, ordering plan, and aggregate counts
  - MergeExecuteResult.was_conflict flag for programmatic conflict detection
  - MergeExecuteResult.conflict_details provides ConflictScan with line-level marker locations
  - MergeSessionResult.error carries conflict file lists or handler decision messages
  - MergeRunnerError provides actionable pre-flight failure messages (dirty tree, in-progress merge)
  - ConflictScan.truncated indicates if file scan was capped at 100 files
drill_down_paths:
  - .kata/milestones/M002/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M002/slices/S03/tasks/T03-SUMMARY.md
duration: 40m
verification_result: passed
completed_at: 2026-03-17
---

# S03: Sequential Merge Runner & Conflict Contract

**Sequential merge runner merges completed session branches in topological order with configurable ordering strategies, closure-based conflict handling, and structured MergeReport output.**

## What Happened

Built the complete merge infrastructure in three tasks:

**T01** extended `assay-types/src/merge.rs` with `MergeExecuteResult`, `ConflictScan`, and `ConflictMarker` types. Implemented `merge_execute()` in `assay-core/src/merge.rs` — checks for in-progress merge (MERGE_HEAD), runs `git merge --no-ff`, on success extracts merge SHA and changed files, on conflict collects conflicting files, scans for markers, runs `--abort`, and returns structured result. Added `scan_conflict_markers()` and `scan_files_for_markers()` with a 100-file cap. Changed `git_raw()`/`git_command()` to `pub(crate)`. Added `MergeExecuteError` variant to `AssayError`. Integration tests use real git repos via tempfile.

**T02** added all serializable merge report types to `assay-types/src/orchestrate.rs`: `MergeStrategy`, `MergePlan`, `MergePlanEntry`, `MergeSessionStatus`, `MergeSessionResult`, `MergeReport`, `ConflictAction` — all with `deny_unknown_fields`, schemars, inventory registration. Created `ordering.rs` with `CompletedSession` (operational, not serializable), `order_sessions()` dispatching to `CompletionTime` (sort by timestamp, topo tiebreak, name tiebreak) and `FileOverlap` (greedy least-overlap-first with merged file set tracking) strategies. 10 schema snapshots locked, 8 ordering unit tests.

**T03** created `merge_runner.rs` with `merge_completed_sessions()` — the capstone function. Pre-flight validates clean working tree and no MERGE_HEAD. Orders sessions via `order_sessions()`. Iterates: merge each session, invoke conflict handler on conflicts, record result. Abort flag stops remaining sessions. Provides `default_conflict_handler()` (returns Skip) and `extract_completed_sessions()` (bridges OrchestratorResult to CompletedSession). Added `MergeRunnerError` variant. 6 integration tests with real git repos: 3-way clean merge, conflict with skip, conflict with abort, empty sessions, dirty tree error, branch name derivation.

## Verification

- `cargo test -p assay-core --features orchestrate` — 739 passed (21 new: 7 merge execution + 8 ordering + 6 merge runner)
- `cargo test -p assay-types --features orchestrate` — 156 passed across lib/integration/snapshot suites (10 new schema snapshots)
- `cargo clippy -p assay-core -p assay-types --features orchestrate -- -D warnings` — clean
- `just ready` — all checks passed (fmt, lint, test, deny)

## Requirements Advanced

- R023 — Sequential merge runner delivered: `merge_completed_sessions()` merges branches in topological order with conflict detection against updated base, closure-based conflict handler, configurable ordering strategies, and structured MergeReport

## Requirements Validated

- R023 — Integration tests with real git repos prove: topological merge ordering correctness, conflict detection against updated base after each merge, conflict handler invocation, abort propagation, MergeReport accuracy with per-session status and aggregate counts. `just ready` passes.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None.

## Known Limitations

- `extract_completed_sessions()` derives branch names from session names via `assay/<slug>` when `branch_name` is empty — S06 should populate real branch names from executor worktrees
- AI conflict resolution deferred to M003 (R026) — default handler only returns Skip
- 2 pre-existing test failures in `assay-mcp` crate (unrelated to S03 changes) — these are from earlier milestone work

## Follow-ups

- S06 wires `merge_completed_sessions()` into the orchestrator's post-execution phase
- S06 needs to pass real `SessionOutcome::Completed` entries with populated `branch_name` fields

## Files Created/Modified

- `crates/assay-types/src/merge.rs` — added `MarkerType`, `ConflictMarker`, `ConflictScan`, `MergeExecuteResult` types
- `crates/assay-types/src/orchestrate.rs` — added `MergeStrategy`, `MergePlan`, `MergePlanEntry`, `MergeSessionStatus`, `MergeSessionResult`, `MergeReport`, `ConflictAction` types
- `crates/assay-types/src/lib.rs` — added new type exports
- `crates/assay-types/tests/schema_snapshots.rs` — 10 new schema snapshot tests
- `crates/assay-types/tests/snapshots/` — 10 new snapshot files
- `crates/assay-core/src/merge.rs` — `pub(crate)` visibility, `merge_execute()`, `scan_conflict_markers()`, `scan_files_for_markers()`
- `crates/assay-core/src/orchestrate/ordering.rs` — new: `CompletedSession`, `order_sessions()`, CompletionTime/FileOverlap strategies
- `crates/assay-core/src/orchestrate/merge_runner.rs` — new: `MergeRunnerConfig`, `merge_completed_sessions()`, `default_conflict_handler()`, `extract_completed_sessions()`
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod ordering`, `pub mod merge_runner`
- `crates/assay-core/src/error.rs` — added `MergeExecuteError` and `MergeRunnerError` variants

## Forward Intelligence

### What the next slice should know
- `merge_completed_sessions()` accepts any `Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction` — S06 should pass `default_conflict_handler()` for now, M003 will swap in an AI handler
- `extract_completed_sessions()` bridges from `OrchestratorResult` to the merge runner's input — S06 calls this after `run_orchestrated()` completes
- The merge runner operates on the base branch directly (not in worktrees) — the working directory must be the project root with the base branch checked out

### What's fragile
- Branch name derivation in `extract_completed_sessions()` uses a simple slug from session name — if session names contain unusual characters, the derived branch name may not match the actual worktree branch. S06 should populate `branch_name` from the executor's worktree metadata.

### Authoritative diagnostics
- `MergeReport` is the single source of truth for merge outcomes — deserialize it to see per-session status, ordering rationale, and aggregate counts
- `MergeRunnerError` in the error chain gives actionable pre-flight failure messages

### What assumptions changed
- No assumptions changed — implementation matched the plan closely
