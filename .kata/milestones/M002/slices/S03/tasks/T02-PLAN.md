---
estimated_steps: 4
estimated_files: 5
---

# T02: Add merge ordering strategies and orchestrate merge types

**Slice:** S03 — Sequential Merge Runner & Conflict Contract
**Milestone:** M002

## Description

Implement the two merge ordering strategies (completion-time and file-overlap) as pure functions, and define all serializable types needed by the merge runner. Ordering strategies determine the sequence in which completed session branches are merged into the base — this is critical because merging A then B can succeed while B then A conflicts (D019).

`CompletionTime` sorts by completion timestamp with topological tiebreak. `FileOverlap` uses a greedy algorithm: iteratively pick the session whose changed files have the least overlap with the already-merged set, minimizing conflict probability (D025).

All serializable types go in `assay-types/src/orchestrate.rs` (feature-gated). Operational types and ordering logic go in `assay-core/src/orchestrate/ordering.rs`.

## Steps

1. Add serializable types to `crates/assay-types/src/orchestrate.rs`: `MergeStrategy` enum (`CompletionTime`, `FileOverlap`), `MergePlan` (strategy, entries), `MergePlanEntry` (session_name, position, reason), `MergeSessionStatus` enum (Merged, Skipped, ConflictSkipped, Aborted, Failed), `MergeSessionResult` (session_name, status, merge_sha, error), `MergeReport` (sessions_merged, sessions_skipped, conflict_skipped, aborted, plan, results, duration). All with `deny_unknown_fields`, schemars, inventory. Add `ConflictAction` enum (Resolved(String), Skip, Abort) — not `deny_unknown_fields` since it's operational, but needs serde for logging.

2. Add schema snapshot tests for all new types in `crates/assay-types/tests/schema_snapshots.rs`. Run `cargo test -p assay-types --features orchestrate` to generate and lock snapshots.

3. Create `crates/assay-core/src/orchestrate/ordering.rs` with `CompletedSession` struct (session_name, branch_name, changed_files, completed_at, topo_order), `order_sessions(sessions, strategy) -> (Vec<CompletedSession>, MergePlan)` function. Implement `CompletionTime` sorting and `FileOverlap` greedy algorithm. Wire `pub mod ordering` in `orchestrate/mod.rs`.

4. Write unit tests in `ordering.rs`: completion-time sorts by timestamp with topo tiebreak, file-overlap prefers sessions with fewer overlapping files, single session returns unchanged, empty sessions returns empty, tie-breaking is deterministic.

## Must-Haves

- [ ] All serializable merge report types in assay-types with full derives, `deny_unknown_fields`, inventory, schema snapshots
- [ ] `ConflictAction` enum with `Resolved(String)`, `Skip`, `Abort` variants
- [ ] `CompletionTime` ordering: sort by completion timestamp, topological index as tiebreak
- [ ] `FileOverlap` ordering: greedy least-overlap-first algorithm
- [ ] `order_sessions()` returns both ordered sessions and a `MergePlan` for observability
- [ ] Unit tests for both strategies with deterministic assertions

## Verification

- `cargo test -p assay-types --features orchestrate -- orchestrate` — type round-trip and schema tests pass
- `cargo test -p assay-core --features orchestrate -- orchestrate::ordering` — ordering strategy tests pass
- `cargo clippy -p assay-core -p assay-types --features orchestrate -- -D warnings` — no warnings

## Observability Impact

- Signals added/changed: `MergePlan` provides per-session ordering rationale (position in sequence, reason for placement); `MergeReport` provides complete merge phase summary with per-session status and totals
- How a future agent inspects this: deserialize `MergeReport` from JSON to see which sessions merged, which were skipped, and why; `MergePlan.entries` shows the ordering decision for each session
- Failure state exposed: `MergeSessionResult.error` carries the failure message; `MergeSessionStatus` distinguishes merged/skipped/conflict-skipped/aborted/failed

## Inputs

- `crates/assay-types/src/orchestrate.rs` — existing `SessionRunState`, `FailurePolicy`, `OrchestratorPhase`, `SessionStatus`, `OrchestratorStatus` as pattern reference for derives and inventory
- `crates/assay-core/src/orchestrate/dag.rs` — `DependencyGraph::topological_groups()` for topological ordering reference
- S02 Summary forward intelligence — `OrchestratorResult.outcomes` is `Vec<(String, SessionOutcome)>`

## Expected Output

- `crates/assay-types/src/orchestrate.rs` — 8+ new types with full derives and inventory registration
- `crates/assay-types/tests/schema_snapshots.rs` — new feature-gated snapshot tests
- `crates/assay-types/tests/snapshots/` — new `.snap` files
- `crates/assay-core/src/orchestrate/ordering.rs` — new: `CompletedSession`, `order_sessions()`, unit tests
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod ordering`
