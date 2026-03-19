---
estimated_steps: 5
estimated_files: 2
---

# T02: DependencyGraph construction and validation

**Slice:** S01 — Manifest Dependencies & DAG Validation
**Milestone:** M002

## Description

Implement the `DependencyGraph` struct and `from_manifest()` constructor with full validation: effective name resolution, unique name enforcement, dependency reference resolution, self-dependency rejection, missing reference rejection, and cycle detection via Kahn's algorithm. This is the central data structure that S02 (executor), S03 (merge runner), and S06 (CLI routing) all depend on. Port semantics from Smelt's `orchestrate/dag.rs` but use `Vec<Vec<usize>>` adjacency lists instead of petgraph.

## Steps

1. Define `DependencyGraph` struct in `dag.rs`: `names: Vec<String>` (effective names indexed by session position), `forward_edges: Vec<Vec<usize>>` (idx → dependents), `reverse_edges: Vec<Vec<usize>>` (idx → dependencies), `session_count: usize`. Add basic accessors: `session_count()`, `name_of(idx) -> &str`, `index_of(name) -> Option<usize>`.
2. Implement `from_manifest(&RunManifest) -> Result<DependencyGraph, AssayError>`: compute effective names via `session.name.clone().unwrap_or_else(|| session.spec.clone())`. When any session has non-empty `depends_on`, validate unique effective names. Resolve each `depends_on` reference to an index. Reject self-dependencies and missing references.
3. Implement Kahn's cycle detection within `from_manifest()`: compute in-degree array, seed BFS queue with zero-in-degree nodes, process until queue empty. If processed count < session count, collect unprocessed session names and return `AssayError::DagCycle { sessions }`.
4. Add error variants to `crates/assay-core/src/error.rs`: `DagCycle { sessions: Vec<String> }` and `DagValidation { errors: Vec<ManifestError> }`. Use `ManifestError` from `crate::manifest` for structured validation errors.
5. Write unit tests (10+): valid linear chain (a→b→c), valid diamond (a→{b,c}→d), valid parallel (no deps), valid single session, cycle detection (a→b→a) with session names in error, missing dependency reference, self-dependency, duplicate effective names with deps, empty depends_on treated as no deps, mixed — some sessions with deps some without.

## Must-Haves

- [ ] `DependencyGraph` struct with adjacency list representation
- [ ] `from_manifest()` resolves effective names and builds graph
- [ ] Cycle detection names the participating sessions
- [ ] Missing dependency references produce errors naming the unknown session
- [ ] Self-dependency rejected with clear error
- [ ] Duplicate effective names rejected when dependencies present
- [ ] Index stability — `names[i]` corresponds to `manifest.sessions[i]`
- [ ] 10+ unit tests covering all validation cases

## Verification

- `cargo test -p assay-core --features orchestrate` — all DAG construction tests pass
- Error messages verified by test assertions to contain specific session names

## Observability Impact

- Signals added/changed: Structured error variants (`DagCycle`, `DagValidation`) with session names for actionable diagnostics
- How a future agent inspects this: error messages from `from_manifest()` name exact sessions causing the problem
- Failure state exposed: cycle participant names, missing ref name, duplicate pair

## Inputs

- `crates/assay-types/src/manifest.rs` — `ManifestSession` with `depends_on` field (from T01)
- `crates/assay-core/src/orchestrate/dag.rs` — placeholder from T01
- `crates/assay-core/src/error.rs` — existing `AssayError` enum
- Smelt reference: `../smelt/crates/smelt-core/src/orchestrate/dag.rs` — `build_dag()` semantics

## Expected Output

- `crates/assay-core/src/orchestrate/dag.rs` — complete `DependencyGraph` struct with `from_manifest()`, accessors, and 10+ tests
- `crates/assay-core/src/error.rs` — `DagCycle` and `DagValidation` error variants
