# S01: Manifest Dependencies & DAG Validation

**Goal:** Extend the manifest type with `depends_on` session dependencies, build a `DependencyGraph` in a feature-gated `orchestrate` module, and validate dependency graphs (cycles, missing refs, duplicates) with actionable errors. Establish the feature gate pattern all M002 slices follow.
**Demo:** User authors a multi-session manifest with `depends_on` fields. A test program builds the DAG, prints topological parallelism groups, and rejects invalid graphs with clear error messages naming the offending sessions. Existing single-session manifests parse identically to before.

## Must-Haves

- `ManifestSession.depends_on: Vec<String>` field exists with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- Existing manifests without `depends_on` parse without error (backward compatibility)
- Schema snapshots updated and locked for `manifest-session-schema` and `run-manifest-schema`
- `assay-core::orchestrate` module exists behind `cfg(feature = "orchestrate")`
- `DependencyGraph::from_manifest()` builds an adjacency-list graph from session dependencies
- Cycle detection rejects circular dependencies and names the sessions involved
- Missing dependency references produce actionable errors naming the unknown session
- Duplicate effective names produce actionable errors when dependencies are present
- Self-dependency rejected
- `ready_set()` returns sessions whose dependencies are all completed or skipped
- `mark_skipped_dependents()` BFS-marks transitive dependents of a failed session
- `topological_groups()` returns parallelism groups for merge ordering
- `cargo check` passes with and without `--features orchestrate`
- `just ready` passes

## Proof Level

- This slice proves: contract (DAG construction, validation, and query are correct per unit tests)
- Real runtime required: no (pure data structures, no I/O, no process spawning)
- Human/UAT required: no

## Verification

- `cargo test -p assay-core --features orchestrate` — all DAG unit tests pass
- `cargo test -p assay-core` (without feature) — existing tests pass, orchestrate module absent
- `cargo test -p assay-types` — schema snapshots match, round-trip tests pass with new field
- `cargo insta test --review` — no pending snapshot changes
- `just ready` — full suite green

## Observability / Diagnostics

- Runtime signals: `DependencyGraph::from_manifest()` returns `Result` with structured error variants naming specific sessions — no logging in this slice (pure logic)
- Inspection surfaces: none (this is a library module, not a service)
- Failure visibility: error messages include session names and dependency references that caused the failure (cycle participants, missing ref name, duplicate name pair)
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `assay-types::ManifestSession` (adding `depends_on` field), `assay-core::manifest::ManifestError` (reusing for DAG validation errors)
- New wiring introduced in this slice: `assay-core::orchestrate::dag` module (feature-gated); `depends_on` field on existing type
- What remains before the milestone is truly usable end-to-end: S02 (executor uses `ready_set`/`mark_skipped_dependents`), S03 (merge runner uses `topological_groups`), S06 (CLI routing detects multi-session manifests and invokes orchestrator)

## Tasks

- [x] **T01: Add depends_on field and establish feature gate** `est:30m`
  - Why: The type evolution is the riskiest part — if backward compat breaks, everything downstream is blocked. Also establishes the feature gate pattern for all M002 work.
  - Files: `crates/assay-types/src/manifest.rs`, `crates/assay-core/Cargo.toml`, `crates/assay-core/src/lib.rs`, `crates/assay-core/src/orchestrate/mod.rs`
  - Do: Add `depends_on: Vec<String>` to `ManifestSession` with serde defaults. Add `[features] orchestrate = []` to assay-core Cargo.toml. Create `orchestrate/mod.rs` with `pub mod dag;` and a placeholder `dag.rs`. Wire `#[cfg(feature = "orchestrate")] pub mod orchestrate;` in lib.rs. Run `cargo insta test --review` to accept schema snapshot updates. Verify existing manifest tests still pass.
  - Verify: `cargo test -p assay-types`, `cargo test -p assay-core`, `cargo test -p assay-core --features orchestrate`, `cargo check -p assay-core --no-default-features` all pass
  - Done when: `depends_on` parses from TOML, existing manifests unaffected, orchestrate module compiles with and without feature, schema snapshots accepted

- [x] **T02: DependencyGraph construction and validation** `est:45m`
  - Why: The core data structure and validation logic that every downstream slice depends on. Ports Smelt's `build_dag` semantics into Assay's conventions (no petgraph, closures not traits, Vec-index addressing).
  - Files: `crates/assay-core/src/orchestrate/dag.rs`, `crates/assay-core/src/error.rs`
  - Do: Implement `DependencyGraph` struct with: `names: Vec<String>` (effective names), `forward_edges: Vec<Vec<usize>>` (dependents), `reverse_edges: Vec<Vec<usize>>` (dependencies). Implement `from_manifest(&RunManifest) -> Result<DependencyGraph>` that: (1) computes effective names, (2) validates unique effective names when any session uses depends_on, (3) resolves depends_on references to indices, (4) rejects self-dependencies, (5) rejects missing references, (6) runs Kahn's algorithm for cycle detection. Add `AssayError` variants: `DagCycle { sessions: Vec<String> }`, `DagValidation { errors: Vec<ManifestError> }`. Add accessors: `session_count()`, `name_of(idx)`, `index_of(name)`. Write 10+ unit tests covering: valid DAG, linear chain, diamond, cycle detection with session names, missing ref, self-dep, duplicate names, single session, empty depends_on.
  - Verify: `cargo test -p assay-core --features orchestrate` — all DAG construction/validation tests pass
  - Done when: `DependencyGraph::from_manifest()` correctly builds graphs and rejects all invalid cases with actionable error messages

- [x] **T03: Ready-set, skip-dependents, and topological groups** `est:30m`
  - Why: These are the query methods downstream slices call — S02's executor loop calls `ready_set` and `mark_skipped_dependents`, S03's merge runner calls `topological_groups`. Without these, the DAG is a validated but inert data structure.
  - Files: `crates/assay-core/src/orchestrate/dag.rs`
  - Do: Implement `ready_set(&self, completed: &HashSet<usize>, in_flight: &HashSet<usize>, skipped: &HashSet<usize>) -> Vec<usize>` — returns session indices not in any set whose reverse_edges are all in completed∪skipped. Implement `mark_skipped_dependents(&self, failed_idx: usize, skipped: &mut HashSet<usize>)` — BFS through forward_edges marking transitive dependents. Implement `topological_groups(&self) -> Vec<Vec<usize>>` — layer-by-layer Kahn's returning parallelism groups. Write 10+ unit tests covering: roots returned first, completion unblocks dependents, skipped dep satisfies, in-flight excluded, transitive skip propagation, partial skip (independent unaffected), topological groups for linear/diamond/parallel graphs.
  - Verify: `cargo test -p assay-core --features orchestrate` — all query method tests pass
  - Done when: All three methods return correct results for every graph shape tested, matching Smelt's `ready_set` and `mark_skipped_dependents` semantics

- [x] **T04: Manifest validation integration and just ready** `est:20m`
  - Why: Validates that the full build pipeline works with the new code — feature gate enabled in downstream crates, `just ready` passes, no regressions in MCP or CLI compilation.
  - Files: `crates/assay-cli/Cargo.toml`, `crates/assay-mcp/Cargo.toml`, `crates/assay-core/src/manifest.rs`, `justfile`
  - Do: Enable `orchestrate` feature in assay-cli and assay-mcp Cargo.toml dependencies (`assay-core = { workspace = true, features = ["orchestrate"] }`). Add dependency-aware validation to `assay-core::manifest::validate()` — when any session has non-empty `depends_on`, check that all references resolve to effective names of other sessions (lightweight pre-check before full DAG construction). Verify `just ready` passes (fmt, lint, test, deny). Check if MCP schema snapshots need updating from the type change.
  - Verify: `just ready` passes clean
  - Done when: Full workspace compiles and passes all checks with orchestrate feature enabled in CLI and MCP crates

## Files Likely Touched

- `crates/assay-types/src/manifest.rs` — add `depends_on` field
- `crates/assay-core/Cargo.toml` — add `[features] orchestrate = []`
- `crates/assay-core/src/lib.rs` — add feature-gated `pub mod orchestrate`
- `crates/assay-core/src/orchestrate/mod.rs` — new module root
- `crates/assay-core/src/orchestrate/dag.rs` — DependencyGraph implementation
- `crates/assay-core/src/error.rs` — new error variants
- `crates/assay-core/src/manifest.rs` — dependency-aware validation
- `crates/assay-cli/Cargo.toml` — enable orchestrate feature
- `crates/assay-mcp/Cargo.toml` — enable orchestrate feature
- `crates/assay-types/tests/schema_snapshots.rs` — snapshot updates (automatic)
