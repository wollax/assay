---
id: T02
parent: S01
milestone: M002
provides:
  - DependencyGraph struct with Vec<Vec<usize>> adjacency list representation
  - from_manifest() constructor with full validation (unique names, missing refs, self-deps, cycles)
  - DagCycle and DagValidation error variants on AssayError (feature-gated)
key_files:
  - crates/assay-core/src/orchestrate/dag.rs
  - crates/assay-core/src/error.rs
key_decisions:
  - "Used Vec<Vec<usize>> adjacency lists instead of petgraph — zero additional dependencies"
  - "Validation only enforces unique effective names when at least one session has depends_on — allows duplicate spec refs in manifests without deps"
  - "DagCycle and DagValidation variants are feature-gated with cfg(feature = orchestrate) to avoid exposing them without the feature"
patterns_established:
  - "Kahn's algorithm for cycle detection — returns unprocessed session names in DagCycle error"
  - "from_manifest() collects all validation errors before returning DagValidation, except duplicate names which bail early since reference resolution is impossible"
observability_surfaces:
  - "DagCycle error names cycle participant sessions; DagValidation errors name specific sessions and dependency references"
duration: 1 step
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: DependencyGraph construction and validation

**Implemented DependencyGraph with from_manifest() constructor, full validation (cycles, missing refs, self-deps, duplicates), and 13 unit tests.**

## What Happened

Built the `DependencyGraph` struct in `crates/assay-core/src/orchestrate/dag.rs` with `Vec<Vec<usize>>` forward and reverse adjacency lists. The `from_manifest()` constructor:

1. Computes effective names (`name` if set, otherwise `spec`) with index stability (`names[i]` = `manifest.sessions[i]`)
2. Short-circuits to a trivial edgeless graph when no session declares `depends_on`
3. When dependencies exist: validates unique effective names, resolves each dependency reference to an index, rejects self-dependencies and missing references
4. Runs Kahn's algorithm for cycle detection, collecting unprocessed session names on failure

Added two feature-gated error variants to `AssayError`: `DagCycle { sessions }` and `DagValidation { errors }`.

## Verification

- `cargo test -p assay-core --features orchestrate -- dag` — 13 DAG tests pass (+ 2 pre-existing)
- `cargo test -p assay-core` (without feature) — existing tests pass, orchestrate module absent
- `cargo test -p assay-types` — passes
- `just ready` — full suite green (fmt, lint, test, deny)

## Diagnostics

- `DagCycle` error: `dependency cycle detected among sessions: b, c` — names exact participants
- `DagValidation` error: bullet-list format with field path and message per error, e.g. `sessions[1].depends_on: session 'b' depends on unknown session 'nonexistent'`
- All error messages include the session effective name that caused the problem

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/dag.rs` — Complete DependencyGraph struct with from_manifest(), accessors, and 13 unit tests
- `crates/assay-core/src/error.rs` — Added DagCycle and DagValidation error variants (feature-gated)
