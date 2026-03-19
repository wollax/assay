---
id: T03
parent: S03
milestone: M002
provides:
  - merge_completed_sessions() function for sequential merge of session branches
  - MergeRunnerConfig struct for strategy/project_root/base_branch configuration
  - default_conflict_handler() returning ConflictAction::Skip
  - extract_completed_sessions() bridging from OrchestratorResult outcomes
key_files:
  - crates/assay-core/src/orchestrate/merge_runner.rs
  - crates/assay-core/src/orchestrate/mod.rs
  - crates/assay-core/src/error.rs
key_decisions:
  - Conflict handler closure receives (session_name, conflicting_files, scan, work_dir) — conflicting files extracted from ConflictScan markers and deduplicated
  - MergeRunnerError added as a new AssayError variant for pre-flight errors (dirty tree, in-progress merge)
  - slug_from_name derives branch names by lowercasing and replacing non-alphanumeric chars with hyphens
patterns_established:
  - Pre-flight validation (clean working tree + no MERGE_HEAD) runs before any merge attempt
  - Abort flag propagates through remaining sessions marking them as Aborted
  - ConflictAction::Resolved path records the provided SHA and counts as merged (for future AI resolution)
observability_surfaces:
  - MergeReport provides per-session merge status, ordering plan, and aggregate counts
  - MergeSessionResult.error carries conflict file lists or handler decision messages
  - MergeRunnerError provides actionable messages for dirty tree and in-progress merge pre-flight failures
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Implement merge runner sequencing loop with conflict handler

**Built `merge_completed_sessions()` that wires ordering, `merge_execute()`, and closure-based conflict handling into a complete sequential merge loop with pre-flight validation and `MergeReport` output.**

## What Happened

Created `crates/assay-core/src/orchestrate/merge_runner.rs` with the full merge runner implementation:

1. **`MergeRunnerConfig`** — holds strategy, project root, and base branch.
2. **`merge_completed_sessions<H>()`** — the main entry point. Pre-flight validates clean working tree and no in-progress merge, orders sessions via `order_sessions()`, iterates calling `merge_execute()` for each, invokes the conflict handler on conflicts, and builds a `MergeReport` with per-session results and totals.
3. **`default_conflict_handler()`** — returns a closure that always returns `ConflictAction::Skip`.
4. **`extract_completed_sessions()`** — bridges from `OrchestratorResult.outcomes` to `Vec<CompletedSession>`, deriving branch names from session names using `assay/<slug>` pattern when `branch_name` is empty.

Added `MergeRunnerError` variant to `AssayError` for pre-flight validation errors.

Wired `pub mod merge_runner` in `orchestrate/mod.rs`.

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate::merge_runner` — 6 tests pass:
  - `test_merge_three_sessions_no_conflicts` — 3 branches merge cleanly, report shows 3 merged
  - `test_merge_conflict_skip_continues` — middle session conflicts, skip handler invoked, report shows 2 merged + 1 conflict-skipped
  - `test_merge_abort_stops_loop` — abort handler stops loop, report shows 1 merged + 2 aborted
  - `test_merge_empty_sessions` — returns empty report
  - `test_merge_dirty_working_tree_error` — returns error before attempting any merge
  - `test_extract_completed_sessions_derives_branch_names` — empty branch names derive from session names
- `cargo test -p assay-core --features orchestrate -- orchestrate::ordering` — 8 tests pass
- `cargo test -p assay-core --features orchestrate -- merge` — 35 tests pass (includes T01 merge tests + merge runner tests)
- `just fmt-check` — passes
- `just lint` — passes (clippy clean)
- `just test` — 2 pre-existing failures in `assay-mcp` (unrelated to this change: `gate_finalize_invalid_session_returns_error` and `gate_report_and_finalize_not_found_errors_are_consistent`)

## Diagnostics

- Deserialize `MergeReport` to inspect per-session merge outcomes, ordering plan, and aggregate counts
- `MergeSessionResult.status` distinguishes `Merged`, `ConflictSkipped`, `Aborted`, `Failed`, `Skipped`
- `MergeSessionResult.error` carries conflict file lists or handler decision messages
- `MergeRunnerError` in error chain provides actionable pre-flight failure messages (dirty tree, in-progress merge)
- `MergeReport.plan` shows the ordering strategy and per-session placement rationale

## Deviations

None.

## Known Issues

- 2 pre-existing test failures in `assay-mcp` crate (`gate_finalize_invalid_session_returns_error`, `gate_report_and_finalize_not_found_errors_are_consistent`) — not introduced by this task.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/merge_runner.rs` — new: `MergeRunnerConfig`, `merge_completed_sessions()`, `default_conflict_handler()`, `extract_completed_sessions()`, 6 integration tests with real git repos
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod merge_runner`
- `crates/assay-core/src/error.rs` — added `MergeRunnerError` variant to `AssayError`
