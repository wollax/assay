---
estimated_steps: 4
estimated_files: 1
---

# T03: Ready-set, skip-dependents, and topological groups

**Slice:** S01 — Manifest Dependencies & DAG Validation
**Milestone:** M002

## Description

Implement the three query methods on `DependencyGraph` that downstream slices depend on: `ready_set()` for the executor's dispatch loop (S02), `mark_skipped_dependents()` for failure propagation (S02), and `topological_groups()` for merge ordering (S03). These transform the validated DAG from an inert data structure into the scheduling engine's core. Port semantics from Smelt's `ready_set` and `mark_skipped_dependents` — particularly that a skipped dependency counts as satisfied (so dependents of skipped sessions still become ready).

## Steps

1. Implement `ready_set(&self, completed: &HashSet<usize>, in_flight: &HashSet<usize>, skipped: &HashSet<usize>) -> Vec<usize>`: iterate all session indices, exclude those in completed/in_flight/skipped, include those whose every reverse_edge entry is in completed∪skipped. Return as sorted Vec for determinism.
2. Implement `mark_skipped_dependents(&self, failed_idx: usize, skipped: &mut HashSet<usize>)`: BFS from `failed_idx` through `forward_edges`. For each reachable node not already in skipped, insert into skipped and continue BFS. Do NOT add `failed_idx` itself to skipped (caller handles that).
3. Implement `topological_groups(&self) -> Vec<Vec<usize>>`: layer-by-layer Kahn's — start with all zero-in-degree nodes as layer 0, decrement in-degrees for their dependents, collect next zero-in-degree set as layer 1, repeat. Each layer is a Vec of indices that can execute concurrently. Return `Vec<Vec<usize>>`. This naturally produces the correct merge ordering for S03.
4. Write unit tests (10+): ready_set returns roots when nothing completed, completion unblocks dependents, skipped dep satisfies (Smelt semantics), in-flight excluded from ready, mark_skipped transitive (a→b→c: fail a → skip b,c), mark_skipped partial (a→b, c independent: fail a → skip b only, not c), mark_skipped doesn't add failed node itself, topological_groups linear (3 layers of 1), topological_groups diamond (a, then {b,c}, then d), topological_groups fully parallel (1 layer of N), ready_set empty when all completed or in_flight.

## Must-Haves

- [ ] `ready_set()` returns correct sessions based on completed/in_flight/skipped sets
- [ ] Skipped dependency counts as satisfied (Smelt semantics)
- [ ] `mark_skipped_dependents()` BFS marks transitive dependents
- [ ] `mark_skipped_dependents()` does not mark the failed node itself
- [ ] `topological_groups()` returns correct parallelism layers
- [ ] All return values are deterministically ordered (sorted indices)
- [ ] 10+ unit tests covering all edge cases

## Verification

- `cargo test -p assay-core --features orchestrate` — all query method tests pass
- Test names follow pattern `test_ready_set_*`, `test_mark_skipped_*`, `test_topological_groups_*`

## Observability Impact

- Signals added/changed: None — pure query methods on an immutable graph, no I/O
- How a future agent inspects this: unit tests document expected behavior for every graph shape
- Failure state exposed: None (methods don't fail — they operate on a pre-validated graph)

## Inputs

- `crates/assay-core/src/orchestrate/dag.rs` — `DependencyGraph` struct with `forward_edges`, `reverse_edges` (from T02)
- Smelt reference: `../smelt/crates/smelt-core/src/orchestrate/dag.rs` — `ready_set()`, `mark_skipped_dependents()` semantics

## Expected Output

- `crates/assay-core/src/orchestrate/dag.rs` — three query methods + 10+ unit tests added to existing file
