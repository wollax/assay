# S01: Manifest Dependencies & DAG Validation — Research

**Date:** 2026-03-17
**Domain:** Graph algorithms, TOML schema evolution, manifest validation
**Confidence:** HIGH

## Summary

S01 adds `depends_on` to `ManifestSession`, builds a `DependencyGraph` in a new feature-gated `assay-core::orchestrate` module, and validates the graph (cycle detection, missing references, duplicate specs). This is the foundation every downstream M002 slice builds on — S02 uses the ready-set for dispatch, S03 uses topological order for merge sequencing, S05 uses session relationships for scope prompts.

The riskiest assumption is not the algorithm (Kahn's toposort is trivial) but the **type evolution**: adding `depends_on` to `ManifestSession` while maintaining backward compatibility with existing single-session manifests and the 20+ existing schema snapshot tests. The second risk is the **feature gate pattern** — `assay-core` has no existing feature gates, so S01 establishes the pattern that S02–S06 follow.

The primary recommendation is: add `depends_on: Vec<String>` with `#[serde(default)]` to `ManifestSession`, extend the existing `validate()` function in `assay-core::manifest` with dependency validation, and put the `DependencyGraph` struct in a new `assay-core::orchestrate::dag` module behind `cfg(feature = "orchestrate")`. Hand-roll Kahn's algorithm (~40 lines). Port `ready_set` and `mark_skipped_dependents` semantics from Smelt's `orchestrate/dag.rs` but use index-based addressing (Vec indices, not petgraph NodeIndex) since we're not using petgraph.

## Recommendation

**Type change:** Add `depends_on: Vec<String>` to `ManifestSession` in `assay-types/src/manifest.rs` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. This preserves backward compatibility — existing TOML manifests without `depends_on` still parse. The `deny_unknown_fields` attribute is already on `ManifestSession`, so this is a controlled schema evolution. Schema snapshots for `run-manifest-schema` and `manifest-session-schema` will need updating via `cargo insta review`.

**Session identity:** Smelt requires `name` as a mandatory field. Assay has `name: Option<String>` with `spec` as the mandatory field. For dependency references, use the *effective name*: `session.name.unwrap_or(session.spec.clone())`. This matches the existing pattern in `pipeline.rs` line 316. Validation must enforce unique effective names when `depends_on` is used by any session.

**Module structure:** Create `crates/assay-core/src/orchestrate/mod.rs` and `crates/assay-core/src/orchestrate/dag.rs` behind `cfg(feature = "orchestrate")`. The feature gate goes in `assay-core/Cargo.toml` as `[features] orchestrate = []` (no extra deps). `assay-cli` and `assay-mcp` enable it. `assay-types` does NOT need the feature — types are always available.

**Graph representation:** Simple adjacency list using session indices (0..N) as node IDs. No petgraph, no heap-allocated NodeIndex. A `Vec<Vec<usize>>` for forward edges (dependents) and `Vec<usize>` for in-degree counts. This is ~40 lines for the full Kahn's implementation including cycle detection.

**Validation location:** Dependency-specific validation (cycles, missing refs, self-deps, duplicate effective names) belongs in the new `orchestrate::dag` module, not in the existing `assay-core::manifest::validate()`. Rationale: the existing validate() is behind no feature gate and handles structural TOML issues. DAG validation is semantic and only matters when the orchestrate feature is enabled. However, the `depends_on` field parsing works without the feature gate (it's just a Vec<String> on the type).

## What Should Be Proven First

The riskiest thing is the **type evolution + backward compatibility**. If adding `depends_on` to `ManifestSession` breaks existing manifest parsing, every downstream slice is blocked. This should be the first task: add the field, update snapshots, verify existing tests still pass.

The second risk is the **feature gate mechanics**. Assay has no existing feature gates on `assay-core`. Getting `cfg(feature = "orchestrate")` working correctly — compilation with and without the feature, downstream crates enabling it — establishes the pattern for all M002 work.

## What Existing Patterns Should Be Reused

- **Manifest validation pattern:** `assay-core::manifest::validate()` collects all errors into `Vec<ManifestError>` for single-pass fixing. DAG validation should return the same `Vec<ManifestError>` type so errors compose.
- **Error type pattern:** `AssayError` is `#[non_exhaustive]` with thiserror. New variants for DAG errors follow the same pattern as `ManifestParse`/`ManifestValidation`.
- **Schema snapshot pattern:** Every new type in `assay-types` gets an `inventory::submit!` registration and a schema snapshot test in `crates/assay-types/tests/schema_snapshots.rs`.
- **Effective name pattern:** `pipeline.rs:316` uses `session.name.clone().unwrap_or_else(|| session.spec.clone())` — this is the canonical way to get a session's identity.

## What Boundary Contracts Matter

The `DependencyGraph` is the central contract. Downstream slices need:

1. **`ready_set(completed, in_flight, skipped) -> Vec<usize>`** — S02 calls this in a loop to dispatch sessions. Must handle: all deps completed, skipped dep counts as satisfied (Smelt semantics), in-flight excluded.
2. **`mark_skipped_dependents(failed_idx, skipped) -> ()`** — S02 calls this on failure. BFS through forward edges.
3. **`topological_groups() -> Vec<Vec<usize>>`** — S03 needs this for merge ordering. Each inner Vec is a set of sessions that can run concurrently (same topological level).
4. **`session_count()`, `name_of(idx)`** — Basic accessors.
5. **Index stability** — Session indices correspond to `manifest.sessions[idx]`. No reordering.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Topological sort with cycle detection | Kahn's algorithm (~40 lines) | Textbook. Graph has ≤20 nodes. No external dep needed. Port semantics from Smelt's `build_dag` + `ready_set`. |
| TOML schema evolution | `#[serde(default, skip_serializing_if)]` | Proven backward-compat pattern already used on ManifestSession for `hooks`, `prompt_layers`. |
| Validation error collection | Existing `Vec<ManifestError>` pattern | `manifest::validate()` already does single-pass error collection. Extend, don't reinvent. |
| Feature gating | `cfg(feature = "...")` in Cargo.toml | Standard Rust pattern. No macro or proc-macro needed. |

## Existing Code and Patterns

- **`crates/assay-types/src/manifest.rs`** — `ManifestSession` struct with `deny_unknown_fields`. Adding `depends_on` here. Has `inventory::submit!` for schema registry. `name: Option<String>` already exists — this is the identity field for dependency references.
- **`crates/assay-core/src/manifest.rs`** — `from_str()`, `validate()`, `load()` functions. `validate()` collects `Vec<ManifestError>`. This is where dependency-aware validation could live (but better in `orchestrate::dag` behind the feature gate).
- **`crates/assay-core/src/lib.rs`** — Module registry. New `pub mod orchestrate;` goes here, gated by `#[cfg(feature = "orchestrate")]`.
- **`crates/assay-core/src/error.rs`** — `AssayError` with `#[non_exhaustive]`. New variants: `DagCycle`, `DagMissingDependency`, `DagDuplicateSession`. Follow `ManifestParse` pattern.
- **`crates/assay-core/src/pipeline.rs:316`** — `session.name.clone().unwrap_or_else(|| session.spec.clone())` — effective name pattern to reuse.
- **`crates/assay-types/tests/schema_snapshots.rs`** — Schema snapshot pattern. Adding `depends_on` will change the `manifest-session-schema` and `run-manifest-schema` snapshots.
- **Smelt reference: `../smelt/crates/smelt-core/src/orchestrate/dag.rs`** — 368 lines including tests. Key functions to port semantics from: `build_dag()` (adjacency list construction + cycle check), `ready_set()` (filter by completed/in_flight/skipped), `mark_skipped_dependents()` (BFS through outgoing edges), `node_by_name()`. Smelt uses petgraph's `DiGraph<String, ()>` — we replace with `Vec<Vec<usize>>` adjacency list.
- **Smelt reference: `../smelt/crates/smelt-core/src/session/manifest.rs`** — `Manifest::validate()` does two-pass validation: first pass checks field-level issues, second pass validates `depends_on` references exist and aren't self-referential. Good pattern to follow.

## Constraints

- **`deny_unknown_fields` on ManifestSession** — Adding `depends_on` is safe because it's a known field. But the attribute means we MUST update schema snapshots; any mismatch breaks tests.
- **No feature gate exists yet on assay-core** — S01 establishes the pattern. Must verify compilation with `--no-default-features` and with `--features orchestrate`.
- **Zero traits (D001)** — `DependencyGraph` is a plain struct with methods, not a trait. Downstream slices take it by reference.
- **Sync core (D007)** — All DAG operations are pure, synchronous, no I/O. This is the easy part.
- **ManifestSession backward compat (D004/R016)** — `depends_on` must use `#[serde(default)]` so existing TOML files still parse. The schema WILL change (new optional field), but deserialization of existing content must not break.
- **Session identity (D021)** — `depends_on` references the effective name (`name` field or `spec` fallback). Uniqueness of effective names is enforced only when dependencies exist (to avoid breaking existing manifests that might have duplicate specs without dependencies).
- **Additive error variants** — `AssayError` is `#[non_exhaustive]`, so adding variants is backward-compatible.

## Common Pitfalls

- **Schema snapshot drift** — Adding `depends_on` changes two schema snapshots (`manifest-session-schema`, `run-manifest-schema`). If you forget to run `cargo insta review`, tests fail with confusing diff output. **How to avoid:** Run `cargo insta test` immediately after adding the field, review and accept new snapshots.
- **Feature gate conditional compilation** — `#[cfg(feature = "orchestrate")]` on `pub mod orchestrate;` in `lib.rs` means the module doesn't exist when the feature is off. Any code outside the feature gate that tries to use `crate::orchestrate::` will fail to compile. **How to avoid:** The `DependencyGraph` and all DAG code live entirely behind the feature gate. Types in `assay-types` (like `depends_on` on ManifestSession) are NOT feature-gated — they're always available.
- **Effective name uniqueness edge case** — Two sessions with `spec = "auth"` but no `name` field have the same effective name "auth". This is fine for single-session manifests but breaks dependency resolution. **How to avoid:** Validate unique effective names only when any session uses `depends_on`. This avoids breaking existing manifests.
- **Smelt's `parallel_by_default` implicit chaining** — Smelt has a `parallel_by_default: bool` that, when false, implicitly chains no-dep sessions sequentially. Assay doesn't need this — it's confusing and unnecessary. If users want sequential execution, they use explicit `depends_on`. **How to avoid:** Don't port this feature. Assay's default is parallel for sessions without dependencies.
- **Cycle detection must name the cycle** — Kahn's algorithm detects cycles but doesn't identify which nodes form the cycle. Smelt's error message is generic ("dependency cycle detected in session DAG"). We can do better: after Kahn's, any nodes not consumed are in a cycle. Report their names. **How to avoid:** After toposort, check if processed count < total count. If so, collect unprocessed node names for the error message.

## Open Risks

- **Schema snapshot update may have cascading effects** — Changing `ManifestSession` schema could affect MCP tool schemas that reference it (via `#[tool_router]` macro). Need to verify the MCP schema snapshots after the type change too.
- **Feature gate interaction with `cargo test`** — `assay-core` tests should run both with and without the `orchestrate` feature. The CI/justfile `just test` command may need `--features orchestrate` or `--all-features` to exercise the new code.

## Smelt Semantics to Port vs. Skip

### Port (adapted to Assay conventions)
- `ready_set()` — filter sessions by completed/in_flight/skipped sets, check all incoming deps satisfied
- `mark_skipped_dependents()` — BFS from failed node through forward edges
- `build_dag()` — adjacency list from manifest with cycle detection
- `node_by_name()` → `index_of(name)` — lookup by effective name

### Skip (not needed for Assay)
- `parallel_by_default: bool` — implicit sequential chaining (confusing, users should be explicit)
- `petgraph` dependency — replaced by hand-rolled adjacency list
- `on_failure` field on ManifestMeta — this becomes a parameter to the executor (S02), not part of DAG validation
- `shared_files`, `task_file`, `script`, `env` fields — these are Smelt-specific manifest features, not needed in S01

## Skills Discovered

No skills are relevant for this slice. The work is standard Rust: struct fields, serde attributes, graph algorithm, unit tests.

## Sources

- Smelt `orchestrate/dag.rs` — reference implementation for ready_set, mark_skipped_dependents semantics (HIGH confidence, reviewed in full)
- Smelt `session/manifest.rs` — reference for depends_on validation pattern (HIGH confidence, reviewed in full)
- Kahn's algorithm — standard topological sort with cycle detection via BFS + in-degree counting. No external source needed.
- Assay codebase — `manifest.rs`, `error.rs`, `pipeline.rs`, `schema_snapshots.rs` patterns (HIGH confidence, read in full)
