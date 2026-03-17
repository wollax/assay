---
id: T03
parent: S01
milestone: M002
provides:
  - "ready_set() query method returning dispatchable session indices based on completed/in_flight/skipped sets"
  - "mark_skipped_dependents() BFS failure propagation through forward_edges"
  - "topological_groups() layer-by-layer Kahn's returning parallelism groups for merge ordering"
key_files:
  - crates/assay-core/src/orchestrate/dag.rs
key_decisions:
  - "Skipped dependencies count as satisfied in ready_set — matches Smelt semantics so dependents of skipped sessions still become ready"
  - "mark_skipped_dependents does not insert failed_idx itself — caller handles failure recording separately from skip propagation"
  - "All return values sorted for determinism (ready_set returns sorted Vec, topological_groups sorts each layer)"
patterns_established:
  - "Query methods on DependencyGraph are pure functions on immutable graph — no mutation, no I/O, no error returns"
  - "BFS via VecDeque for transitive propagation through adjacency lists"
observability_surfaces:
  - none — pure query methods on pre-validated graph, no I/O
duration: 10m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Ready-set, skip-dependents, and topological groups

**Implemented three query methods on DependencyGraph that transform the validated DAG into the scheduling engine's core: ready_set for S02's dispatch loop, mark_skipped_dependents for S02's failure propagation, and topological_groups for S03's merge ordering.**

## What was done

Added three methods to `DependencyGraph` in `crates/assay-core/src/orchestrate/dag.rs`:

1. **`ready_set(&self, completed, in_flight, skipped) -> Vec<usize>`** — iterates all session indices, excludes those in completed/in_flight/skipped, includes those whose every reverse_edge entry is in completed∪skipped. Skipped deps count as satisfied (Smelt semantics).

2. **`mark_skipped_dependents(&self, failed_idx, skipped)`** — BFS through forward_edges from the failed node, inserting all reachable nodes into skipped. Does NOT insert failed_idx itself.

3. **`topological_groups(&self) -> Vec<Vec<usize>>`** — layer-by-layer Kahn's algorithm. Layer 0 is all zero-in-degree nodes, then decrements in-degrees for their dependents, collects next zero-in-degree set, repeat. Each layer sorted for determinism.

## Tests added (15 new tests)

- `test_ready_set_returns_roots_when_nothing_completed`
- `test_ready_set_completion_unblocks_dependents`
- `test_ready_set_skipped_dep_satisfies` (Smelt semantics)
- `test_ready_set_in_flight_excluded`
- `test_ready_set_empty_when_all_completed`
- `test_ready_set_empty_when_all_in_flight`
- `test_ready_set_diamond_needs_both_deps`
- `test_mark_skipped_transitive` (a→b→c chain)
- `test_mark_skipped_partial_independent_unaffected`
- `test_mark_skipped_does_not_add_failed_node`
- `test_mark_skipped_diamond_propagation`
- `test_topological_groups_linear` (3 layers of 1)
- `test_topological_groups_diamond` (a, {b,c}, d)
- `test_topological_groups_fully_parallel` (1 layer of 3)
- `test_topological_groups_single_session`

## Verification

- `cargo test -p assay-core --features orchestrate` — 693 tests pass (15 new query method tests)
- `cargo test -p assay-core` (without feature) — existing tests pass, orchestrate module absent
- `cargo test -p assay-types` — all pass
- `cargo insta test --review` — no pending snapshot changes

## Slice verification status (T03 is task 3 of 4)

| Check | Status |
|-------|--------|
| `cargo test -p assay-core --features orchestrate` | ✅ pass |
| `cargo test -p assay-core` (no feature) | ✅ pass |
| `cargo test -p assay-types` | ✅ pass |
| `cargo insta test --review` | ✅ pass |
| `just ready` | ⏳ deferred to T04 |
