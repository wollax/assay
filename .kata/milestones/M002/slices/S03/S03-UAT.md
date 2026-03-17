# S03: Sequential Merge Runner & Conflict Contract — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All verification uses real git repos created in integration tests — actual `git merge --no-ff` execution, real conflict detection, real `--abort` cleanup. No live runtime or human judgment needed; merge correctness is binary and fully testable.

## Preconditions

- Rust toolchain installed
- `just ready` passes (confirms all deps, formatting, lint, tests, deny)

## Smoke Test

Run `cargo test -p assay-core --features orchestrate -- orchestrate::merge_runner::tests::test_merge_three_sessions_no_conflicts` — should pass, confirming the core merge path works with real git.

## Test Cases

### 1. Clean 3-way merge in topological order

1. Run `cargo test -p assay-core --features orchestrate -- test_merge_three_sessions_no_conflicts`
2. **Expected:** 3 branches merge cleanly into base, MergeReport shows 3 merged / 0 skipped / 0 conflict-skipped

### 2. Conflict with skip handler continues remaining merges

1. Run `cargo test -p assay-core --features orchestrate -- test_merge_conflict_skip_continues`
2. **Expected:** Middle session conflicts, handler returns Skip, remaining sessions merge. Report: 2 merged, 1 conflict-skipped.

### 3. Conflict with abort handler stops loop

1. Run `cargo test -p assay-core --features orchestrate -- test_merge_abort_stops_loop`
2. **Expected:** First conflict triggers abort, remaining sessions marked Aborted. Report: 1 merged, 2 aborted.

### 4. Merge ordering strategies produce correct orderings

1. Run `cargo test -p assay-core --features orchestrate -- orchestrate::ordering`
2. **Expected:** 8 tests pass — CompletionTime sorts by timestamp with topo/name tiebreak, FileOverlap minimizes overlap greedily.

### 5. Conflict marker scanning

1. Run `cargo test -p assay-core --features orchestrate -- scan_conflict_markers`
2. **Expected:** Detects `<<<<<<<`, `=======`, `>>>>>>>` markers with correct file/line/type.

### 6. Schema snapshots locked

1. Run `cargo test -p assay-types --features orchestrate --test schema_snapshots`
2. **Expected:** 50 snapshots pass (10 new for merge types).

## Edge Cases

### Empty completed sessions

1. Run `cargo test -p assay-core --features orchestrate -- test_merge_empty_sessions`
2. **Expected:** Returns empty MergeReport with zero totals, no errors.

### Dirty working tree pre-flight

1. Run `cargo test -p assay-core --features orchestrate -- test_merge_dirty_working_tree_error`
2. **Expected:** Returns MergeRunnerError before attempting any merge.

### Branch name derivation from session names

1. Run `cargo test -p assay-core --features orchestrate -- test_extract_completed_sessions_derives_branch_names`
2. **Expected:** Empty branch names derive `assay/<slug>` from session names.

## Failure Signals

- Any `just ready` failure indicates regression
- Schema snapshot mismatches indicate type contract breakage
- Merge runner tests leaving dirty git state (MERGE_HEAD) indicate abort cleanup failure

## Requirements Proved By This UAT

- R023 (MergeRunner with sequential merge) — Integration tests with real git repos verify topological merge ordering, conflict detection against updated base, closure-based conflict handler invocation, abort propagation, and structured MergeReport accuracy

## Not Proven By This UAT

- End-to-end orchestrator integration (wiring merge_completed_sessions into run_orchestrated post-execution) — deferred to S06
- Real multi-session manifest through the CLI entrypoint — deferred to S06
- AI conflict resolution — deferred to M003 (R026)
- Performance under high branch count (>10 sessions) — not tested

## Notes for Tester

- All tests create real git repos via tempfile — they are fully self-contained and leave no artifacts
- 2 pre-existing test failures in assay-mcp are unrelated to S03; they will be addressed separately
- The conflict handler contract is the key extensibility point — M003 will plug in AI resolution using the same closure signature
