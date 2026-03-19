# S01: Manifest Dependencies & DAG Validation — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is pure data structures and validation logic with no I/O, no runtime behavior, and no user-facing surfaces. Correctness is fully provable through unit tests on graph construction, validation, and query methods. No live runtime or human experience testing is needed.

## Preconditions

- Rust toolchain installed (`cargo` available)
- Repository checked out with S01 changes on the slice branch

## Smoke Test

Run `cargo test -p assay-core --features orchestrate -- dag` and confirm all DAG tests pass.

## Test Cases

### 1. Backward-compatible manifest parsing

1. Parse an existing single-session TOML manifest (no `depends_on` field)
2. **Expected:** Parses successfully with `depends_on` defaulting to empty vec. No error, no schema change visible to the user.

### 2. Valid DAG construction

1. Create a RunManifest with 4 sessions: a (no deps), b (depends_on: [a]), c (depends_on: [a]), d (depends_on: [b, c])
2. Call `DependencyGraph::from_manifest()`
3. **Expected:** Returns Ok with correct adjacency lists. `topological_groups()` returns [[a], [b, c], [d]].

### 3. Cycle detection with actionable error

1. Create a manifest with sessions a→b→c→a (circular)
2. Call `DependencyGraph::from_manifest()`
3. **Expected:** Returns `DagCycle` error naming the cycle participants (a, b, c).

### 4. Missing reference error

1. Create a manifest where session b depends_on "nonexistent"
2. Call `DependencyGraph::from_manifest()`
3. **Expected:** Returns `DagValidation` error naming the unknown session reference.

### 5. Ready-set dispatch scheduling

1. Build a diamond DAG (a→{b,c}→d), mark a as completed
2. Call `ready_set(completed={a}, in_flight={}, skipped={})`
3. **Expected:** Returns [b, c] (both unblocked by a's completion).

### 6. Failure propagation

1. Build a chain a→b→c, mark b as failed
2. Call `mark_skipped_dependents(b, &mut skipped)`
3. **Expected:** skipped contains {c} (transitive dependent). b itself not in skipped.

## Edge Cases

### Self-dependency

1. Create a manifest where session a depends_on ["a"]
2. **Expected:** Rejected with validation error naming the self-referencing session.

### Duplicate effective names with dependencies

1. Create a manifest with two sessions having the same effective name and one has depends_on
2. **Expected:** Rejected with validation error identifying the duplicate.

### No dependencies declared

1. Create a manifest with 3 sessions, none using depends_on
2. Build DependencyGraph
3. **Expected:** Trivial graph with no edges. `topological_groups()` returns one group with all 3 sessions. No unique-name enforcement.

## Failure Signals

- Any `cargo test -p assay-core --features orchestrate` failure
- Existing tests failing when orchestrate feature is off (`cargo test -p assay-core`)
- Schema snapshot mismatches in assay-types
- `just ready` failing on any check

## Requirements Proved By This UAT

- R020 (Multi-agent orchestration) — partially proved: DAG validation foundation (dependency declaration, graph construction, cycle detection, scheduling queries) is correct. Full proof requires S02 (executor) and S06 (integration).

## Not Proven By This UAT

- Runtime orchestration behavior (parallel execution, concurrency control) — requires S02
- Merge ordering correctness with real git operations — requires S03
- End-to-end CLI routing for multi-session manifests — requires S06
- Any I/O, process spawning, or real agent invocation — S01 is pure logic

## Notes for Tester

This slice is entirely unit-testable. The 35 tests in `dag.rs` and 7 tests in `manifest.rs` cover all documented must-haves. Running `cargo test -p assay-core --features orchestrate` is the complete verification — no manual steps needed.
