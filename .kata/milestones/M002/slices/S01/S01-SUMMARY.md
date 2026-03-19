---
id: S01
parent: M002
milestone: M002
provides:
  - "ManifestSession.depends_on: Vec<String> with backward-compatible serde defaults"
  - "DependencyGraph struct with Vec<Vec<usize>> adjacency list representation"
  - "from_manifest() constructor with full validation (cycles, missing refs, self-deps, duplicates)"
  - "ready_set() query for dispatch scheduling"
  - "mark_skipped_dependents() BFS failure propagation"
  - "topological_groups() parallelism groups for merge ordering"
  - "DagCycle and DagValidation error variants on AssayError (feature-gated)"
  - "orchestrate feature gate pattern on assay-core"
  - "Dependency-aware structural validation in manifest::validate()"
requires:
  - slice: none
    provides: "First slice — extends existing ManifestSession/RunManifest types from M001"
affects:
  - S02 (executor uses ready_set, mark_skipped_dependents, DependencyGraph)
  - S03 (merge runner uses topological_groups)
  - S05 (scope enforcement uses ManifestSession.depends_on)
  - S06 (CLI routing uses DAG validation to detect multi-session manifests)
key_files:
  - crates/assay-types/src/manifest.rs
  - crates/assay-core/src/orchestrate/mod.rs
  - crates/assay-core/src/orchestrate/dag.rs
  - crates/assay-core/src/error.rs
  - crates/assay-core/src/manifest.rs
  - crates/assay-core/Cargo.toml
  - crates/assay-cli/Cargo.toml
  - crates/assay-mcp/Cargo.toml
key_decisions:
  - "D024: Hand-rolled Kahn's algorithm with Vec<Vec<usize>> adjacency lists — no petgraph dependency"
  - "D021: depends_on references session effective name (name or spec); unique names enforced only when dependencies are declared"
  - "Feature-gated error variants (DagCycle, DagValidation) with cfg(feature = orchestrate)"
  - "Query methods are pure functions on immutable graph — sorted returns for determinism"
  - "Skipped dependencies count as satisfied in ready_set — matches Smelt semantics"
  - "mark_skipped_dependents does not insert failed_idx — caller handles failure recording"
patterns_established:
  - "Feature gate pattern: `#[cfg(feature = \"orchestrate\")] pub mod orchestrate;` in lib.rs"
  - "Serde backward compat for new Vec fields: `#[serde(default, skip_serializing_if = \"Vec::is_empty\")]`"
  - "BFS via VecDeque for transitive propagation through adjacency lists"
  - "Dependency-aware validation activates only when at least one session has depends_on"
observability_surfaces:
  - "DagCycle error names cycle participant sessions"
  - "DagValidation error uses field paths (sessions[i].depends_on[j]) naming specific problematic references"
  - "manifest::validate() surfaces dependency errors alongside existing ManifestError entries"
drill_down_paths:
  - .kata/milestones/M002/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M002/slices/S01/tasks/T03-SUMMARY.md
  - .kata/milestones/M002/slices/S01/tasks/T04-SUMMARY.md
duration: ~60min across 4 tasks
verification_result: passed
completed_at: 2026-03-17
---

# S01: Manifest Dependencies & DAG Validation

**Extended ManifestSession with `depends_on` dependencies, built a feature-gated DependencyGraph with full validation (cycles, missing refs, self-deps, duplicates) and three query methods (ready_set, mark_skipped_dependents, topological_groups) that S02's executor and S03's merge runner consume directly.**

## What Happened

**T01** added `depends_on: Vec<String>` to `ManifestSession` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for backward compatibility. Established the `orchestrate` feature gate on assay-core and created the `orchestrate/dag.rs` placeholder module. Updated 7 struct literals across manifest.rs and pipeline.rs tests, accepted 2 schema snapshot updates.

**T02** implemented `DependencyGraph` with `Vec<Vec<usize>>` forward/reverse adjacency lists. `from_manifest()` computes effective names, validates uniqueness (only when deps exist), resolves references to indices, rejects self-deps and missing refs, and runs Kahn's algorithm for cycle detection. Added feature-gated `DagCycle` and `DagValidation` error variants to `AssayError`. 13 unit tests cover valid DAGs, linear chains, diamonds, cycles, missing refs, self-deps, duplicates, and single sessions.

**T03** added three query methods: `ready_set()` returns dispatchable sessions based on completed/in_flight/skipped sets (skipped deps count as satisfied per Smelt semantics), `mark_skipped_dependents()` BFS-propagates failure through forward edges, and `topological_groups()` returns layer-by-layer parallelism groups. All returns sorted for determinism. 15 unit tests cover all graph shapes and edge cases.

**T04** enabled the `orchestrate` feature in assay-cli and assay-mcp Cargo.toml, and added dependency-aware structural validation to `manifest::validate()` (unknown refs, self-deps, duplicate names — activates only when deps are present). 7 unit tests added. `just ready` passes clean.

## Verification

- `cargo test -p assay-core --features orchestrate` — 700 tests pass (35 new DAG/validation tests)
- `cargo test -p assay-core` (without feature) — existing tests pass, orchestrate module absent
- `cargo test -p assay-types` — schema snapshots match, round-trip tests pass with new field
- `cargo insta test --review` — no pending snapshot changes
- `just ready` — full suite green (fmt, lint, test, deny)
- `cargo check -p assay-core --no-default-features` — compiles without orchestrate feature

## Requirements Advanced

- R020 (Multi-agent orchestration) — S01 delivers the DAG validation foundation: dependency declaration, graph construction, cycle detection, and scheduling queries. S02 builds the executor on top.

## Requirements Validated

- None moved to validated — R020 requires S02 (executor) and S06 (integration) to complete validation.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

None.

## Known Limitations

- `DependencyGraph` is a pure data structure — no I/O, no process spawning. Runtime orchestration requires S02's executor.
- Duplicate effective names are only checked when at least one session has `depends_on` — manifests without deps allow duplicate spec references (by design, preserving M001 backward compat).
- Validation in `manifest::validate()` mirrors `DependencyGraph::from_manifest()` checks — there is intentional duplication between the lightweight structural pre-check and the full DAG construction.

## Follow-ups

- None discovered during execution.

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — added `depends_on: Vec<String>` field
- `crates/assay-core/Cargo.toml` — added `[features] orchestrate = []`
- `crates/assay-core/src/lib.rs` — added feature-gated `pub mod orchestrate`
- `crates/assay-core/src/orchestrate/mod.rs` — new module root with `pub mod dag`
- `crates/assay-core/src/orchestrate/dag.rs` — complete DependencyGraph with from_manifest(), 3 query methods, 35 tests
- `crates/assay-core/src/error.rs` — added DagCycle and DagValidation error variants (feature-gated)
- `crates/assay-core/src/manifest.rs` — added dependency-aware validation and 7 tests, updated 3 struct literals
- `crates/assay-core/src/pipeline.rs` — updated 4 struct literals with `depends_on: vec![]`
- `crates/assay-cli/Cargo.toml` — enabled orchestrate feature
- `crates/assay-mcp/Cargo.toml` — enabled orchestrate feature
- `crates/assay-types/tests/snapshots/schema_snapshots__manifest-session-schema.snap` — updated
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — updated

## Forward Intelligence

### What the next slice should know
- `DependencyGraph::ready_set()` treats skipped dependencies as satisfied — S02's executor loop should call `mark_skipped_dependents()` first, then `ready_set()` to get the next batch.
- `mark_skipped_dependents()` does NOT insert the failed session itself into skipped — the caller must record the failure separately before propagating skips.
- All query method returns are sorted for determinism — S02 tests can assert exact ordering.

### What's fragile
- The intentional duplication between `manifest::validate()` and `DependencyGraph::from_manifest()` — if validation rules change, both must be updated. The validate() checks are lightweight pre-checks; from_manifest() is authoritative.

### Authoritative diagnostics
- `DependencyGraph::from_manifest()` error messages include session names and field paths — these are the definitive error source for DAG issues.

### What assumptions changed
- None — implementation matched the plan closely. Smelt's semantics ported cleanly to the closure/Vec-index convention.
