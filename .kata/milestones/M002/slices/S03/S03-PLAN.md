# S03: Sequential Merge Runner & Conflict Contract

**Goal:** After parallel execution completes, merge each successful session's branch into the base branch in topological order using `git merge --no-ff`, with configurable ordering strategies and a closure-based conflict handler contract.
**Demo:** Integration tests with real git repos prove that `merge_completed_sessions()` merges N completed session branches in topological order, re-checks conflicts against the updated base after each merge, invokes the conflict handler on conflicts, and produces a `MergeReport` with per-session merge status. Ordering strategies (completion-time and file-overlap) produce correct orderings verified by unit tests.

## Must-Haves

- `merge_execute()` in `merge.rs` performs `git merge --no-ff <branch>` and returns structured result with merge SHA, changed files, or conflict details (with `git merge --abort` on conflict)
- `scan_conflict_markers()` and `scan_files_for_markers()` detect conflict markers in files
- `ordering.rs` provides `CompletionTime` and `FileOverlap` strategies as pure sort functions
- `merge_runner.rs` sequences merges via `merge_completed_sessions()` accepting a conflict handler closure per D001/D026
- `ConflictAction` enum: `Resolved(String)`, `Skip`, `Abort`
- Default conflict handler returns `Skip`
- `MergeReport` captures per-session merge status, plan, and totals
- All new serializable types have `deny_unknown_fields`, schemars, inventory registration, and schema snapshots
- New error variants on `AssayError` for merge execution failures
- Feature-gated behind `orchestrate` for orchestrate-module files; `merge_execute()` in base `merge.rs` is NOT feature-gated

## Proof Level

- This slice proves: integration (real git repos with parallel branches, actual `git merge --no-ff`)
- Real runtime required: yes (real git operations)
- Human/UAT required: no

## Verification

- `cargo test -p assay-core --features orchestrate -- merge::tests` — merge_execute unit/integration tests with real git repos
- `cargo test -p assay-core --features orchestrate -- orchestrate::ordering` — ordering strategy unit tests
- `cargo test -p assay-core --features orchestrate -- orchestrate::merge_runner` — merge runner integration tests with real git repos
- `cargo test -p assay-types --features orchestrate -- merge_runner` — type round-trip and schema tests
- `just ready` — full suite green (fmt, lint, test, deny)

## Observability / Diagnostics

- Runtime signals: `MergeReport` struct provides per-session merge status (merged/skipped/conflict-skipped/aborted), a `MergePlan` showing the chosen ordering, and totals for each outcome class
- Inspection surfaces: `MergeReport` is serializable — S06 can persist it alongside `OrchestratorStatus` for post-run inspection
- Failure visibility: `MergeExecuteResult` on conflict carries conflicting file list and `ConflictScan` with marker locations; `MergeSessionResult` distinguishes merge-failed from conflict-handler-skipped from abort
- Redaction constraints: none — git merge output contains only file paths and commit SHAs

## Integration Closure

- Upstream surfaces consumed:
  - `crates/assay-core/src/orchestrate/executor.rs` → `SessionOutcome::Completed` with `branch_name`, `changed_files`
  - `crates/assay-core/src/orchestrate/dag.rs` → `DependencyGraph::topological_groups()` for merge ordering
  - `crates/assay-core/src/merge.rs` → `git_raw()`, `git_command()` (made `pub(crate)` for `merge_execute()`)
  - `crates/assay-types/src/merge.rs` → existing `MergeCheck`, `MergeConflict`, `ConflictType`
- New wiring introduced in this slice:
  - `merge_execute()` — side-effecting git merge capability in base merge module
  - `merge_completed_sessions()` — sequencing loop composing ordering + merge execution + conflict handling
  - Conflict handler closure contract (`Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction`)
- What remains before the milestone is truly usable end-to-end:
  - S06 wires `merge_completed_sessions()` into the orchestrator's post-execution phase
  - S06 passes real `SessionOutcome::Completed` entries with populated `branch_name` fields (currently placeholder in executor)
  - AI conflict resolution deferred to M003 (R026)

## Tasks

- [x] **T01: Add merge execution types and `merge_execute()` function** `est:35m`
  - Why: Foundation for all merge operations — extends existing `merge.rs` with side-effecting `git merge --no-ff` and conflict marker scanning. All downstream merge work depends on this.
  - Files: `crates/assay-types/src/merge.rs`, `crates/assay-core/src/merge.rs`, `crates/assay-core/src/error.rs`
  - Do: Add `MergeExecuteResult`, `ConflictScan`, `ConflictMarker` types to assay-types. Add `MergeExecuteError` variant to `AssayError`. Make `git_raw()` and `git_command()` `pub(crate)`. Implement `merge_execute()` (checkout base, `git merge --no-ff`, handle conflict with `--abort`, return structured result), `scan_conflict_markers()`, `scan_files_for_markers()`. Write integration tests using real git repos (`tempfile` + `git init`).
  - Verify: `cargo test -p assay-core --features orchestrate -- merge::tests`
  - Done when: `merge_execute()` succeeds on clean merges and returns conflict details on conflicting merges in real git repos, with schema snapshots locked for new types

- [x] **T02: Add merge ordering strategies and orchestrate merge types** `est:25m`
  - Why: Merge runner needs ordering logic and serializable report types before the sequencing loop can be built. Pure functions that are independently testable.
  - Files: `crates/assay-core/src/orchestrate/ordering.rs`, `crates/assay-types/src/orchestrate.rs`, `crates/assay-core/src/orchestrate/mod.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do: Add `CompletedSession`, `MergeStrategy`, `MergePlan`, `MergePlanEntry`, `MergeSessionResult`, `MergeSessionStatus`, `MergeReport`, `ConflictAction` types. Implement `order_sessions()` with `CompletionTime` (sort by timestamp, topological tiebreak) and `FileOverlap` (greedy least-overlap-first) strategies. Add schema snapshots. Wire `pub mod ordering` in orchestrate `mod.rs`.
  - Verify: `cargo test -p assay-core --features orchestrate -- orchestrate::ordering` and `cargo test -p assay-types --features orchestrate`
  - Done when: Both ordering strategies produce correct orderings verified by unit tests; all new types have schema snapshots locked

- [x] **T03: Implement merge runner sequencing loop with conflict handler** `est:35m`
  - Why: The capstone — wires ordering + merge_execute + conflict handler into the complete `merge_completed_sessions()` function. Proves R023 with real git integration tests.
  - Files: `crates/assay-core/src/orchestrate/merge_runner.rs`, `crates/assay-core/src/orchestrate/mod.rs`
  - Do: Implement `merge_completed_sessions()` accepting `OrchestratorResult`, ordering config, and `conflict_handler: impl Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction`. Loop: order sessions → for each, check working tree clean → merge-check or attempt merge → on conflict invoke handler → record result. Check for in-progress merge state at startup. Produce `MergeReport`. Provide `default_conflict_handler()` returning `Skip`. Write integration tests: clean 3-way merge in topological order, conflict with skip handler, conflict with abort handler, re-check after each merge proves stale-base detection, empty completed sessions.
  - Verify: `cargo test -p assay-core --features orchestrate -- orchestrate::merge_runner`
  - Done when: Integration tests with real git repos prove topological merge ordering, conflict detection against updated base, handler invocation, and `MergeReport` accuracy; `just ready` passes

## Files Likely Touched

- `crates/assay-types/src/merge.rs` — new types: `MergeExecuteResult`, `ConflictScan`, `ConflictMarker`
- `crates/assay-types/src/orchestrate.rs` — new types: `CompletedSession`, `MergeStrategy`, `MergePlan`, `MergePlanEntry`, `MergeSessionResult`, `MergeSessionStatus`, `MergeReport`, `ConflictAction`
- `crates/assay-types/tests/schema_snapshots.rs` — new snapshot tests
- `crates/assay-core/src/merge.rs` — `merge_execute()`, `scan_conflict_markers()`, `scan_files_for_markers()`, visibility changes
- `crates/assay-core/src/error.rs` — `MergeExecuteError` variant
- `crates/assay-core/src/orchestrate/ordering.rs` — new: ordering strategies
- `crates/assay-core/src/orchestrate/merge_runner.rs` — new: sequencing loop
- `crates/assay-core/src/orchestrate/mod.rs` — module declarations
