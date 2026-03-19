---
estimated_steps: 4
estimated_files: 3
---

# T03: Implement merge runner sequencing loop with conflict handler

**Slice:** S03 — Sequential Merge Runner & Conflict Contract
**Milestone:** M002

## Description

The capstone task — build `merge_completed_sessions()` that wires together ordering, merge execution, and the conflict handler closure into the complete sequential merge loop. This function takes completed session outcomes from the orchestrator, orders them using the chosen strategy, then for each session: checks working tree is clean, attempts `merge_execute()`, invokes the conflict handler on conflicts, records the result, and continues or aborts based on the handler's response.

This directly proves R023 (MergeRunner with sequential merge) through integration tests with real git repos containing parallel branches that must be merged in topological order.

## Steps

1. Create `crates/assay-core/src/orchestrate/merge_runner.rs`. Define `MergeRunnerConfig` struct (strategy, project_root, base_branch). Implement `merge_completed_sessions<H>(completed_sessions, config, conflict_handler) -> Result<MergeReport>` where `H: Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction`. The function: validates project root has clean working tree (`git status --porcelain`), checks no in-progress merge (`MERGE_HEAD`), orders sessions via `order_sessions()`, iterates in order calling `merge_execute()` for each, on conflict invokes handler and acts on `ConflictAction`, builds `MergeReport` with per-session results and totals.

2. Implement `default_conflict_handler() -> impl Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction` that returns `ConflictAction::Skip`. Implement helper `extract_completed_sessions(outcomes: &[(String, SessionOutcome)]) -> Vec<CompletedSession>` to bridge from `OrchestratorResult` — derive branch names from session names using the `assay/<slug>` pattern when `branch_name` is empty (per research: executor has placeholder values).

3. Wire `pub mod merge_runner` in `orchestrate/mod.rs`. Run `just ready` to verify formatting, linting, and all existing tests still pass.

4. Write integration tests with real git repos in `merge_runner.rs`: (a) 3 sessions with no conflicts — all merge in topological order, report shows 3 merged; (b) middle session conflicts with base after first merge — skip handler invoked, report shows 1 merged + 1 conflict-skipped + 1 merged; (c) abort handler stops merge loop — report shows aborted status; (d) empty completed sessions — returns empty report; (e) dirty working tree — returns error before attempting any merge. Each test creates a real git repo with branches via `tempfile` + `git init` + `git checkout -b` + commits.

## Must-Haves

- [ ] `merge_completed_sessions()` accepts a conflict handler closure per D001/D026
- [ ] Merges execute in topological order with re-check against updated base after each merge
- [ ] Conflict handler receives `(session_name, conflicting_files, scan, work_dir)` and returns `ConflictAction`
- [ ] `Skip` action skips the conflicting session and continues to the next
- [ ] `Abort` action stops the merge loop and marks remaining sessions as aborted
- [ ] `Resolved` action (for future AI resolution) continues to merge attempt
- [ ] `default_conflict_handler()` returns `Skip`
- [ ] Dirty working tree detected before first merge with actionable error
- [ ] `MergeReport` accurately tracks merged/skipped/conflict-skipped/aborted counts
- [ ] `extract_completed_sessions()` derives branch names from session names when empty
- [ ] Integration tests with real git repos prove ordering, conflict handling, and report accuracy
- [ ] `just ready` passes

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate::merge_runner` — all merge runner integration tests pass
- `just ready` — full suite green (fmt, lint, test, deny)

## Observability Impact

- Signals added/changed: `MergeReport` is the primary observability surface for the merge phase — captures per-session outcome, ordering plan, and totals. S06 will persist this alongside `OrchestratorStatus`.
- How a future agent inspects this: deserialize `MergeReport` to see exactly which sessions merged, which had conflicts, what the handler decided, and the final counts
- Failure state exposed: dirty working tree error, in-progress merge error, per-session merge failure with conflicting files

## Inputs

- `crates/assay-core/src/merge.rs` — `merge_execute()` from T01
- `crates/assay-core/src/orchestrate/ordering.rs` — `order_sessions()`, `CompletedSession` from T02
- `crates/assay-types/src/orchestrate.rs` — `MergeReport`, `MergeSessionResult`, `MergeSessionStatus`, `ConflictAction`, `MergePlan` from T02
- `crates/assay-core/src/orchestrate/executor.rs` — `SessionOutcome::Completed` struct for `extract_completed_sessions()`
- S02 forward intelligence: `OrchestratorResult.outcomes` is `Vec<(String, SessionOutcome)>`, `branch_name` is currently `String::new()`

## Expected Output

- `crates/assay-core/src/orchestrate/merge_runner.rs` — new: `MergeRunnerConfig`, `merge_completed_sessions()`, `default_conflict_handler()`, `extract_completed_sessions()`, 5+ integration tests
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod merge_runner`
- R023 proven by integration tests with real git repos
