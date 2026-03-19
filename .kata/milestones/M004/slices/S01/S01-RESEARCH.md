# S01: Mode Infrastructure — Research

**Date:** 2026-03-17

## Summary

S01 is purely additive infrastructure: add `OrchestratorMode`, `MeshConfig`, and `GossipConfig` to `assay-types`, add the `mode`/`mesh_config`/`gossip_config` fields to `RunManifest`, wire mode dispatch at the CLI and MCP call sites, and add `run_mesh()`/`run_gossip()` function stubs in new `assay-core::orchestrate::mesh` and `gossip` modules. No behavioral change to existing DAG execution — this slice just makes `mode = "mesh"` and `mode = "gossip"` parse and route correctly.

The biggest task is the schema snapshot churn: `RunManifest` already has a locked snapshot (`run-manifest-schema.snap`). Adding three new optional fields with `serde(default)` will change the schema and fail the snapshot test. The fix is straightforward: add the types and fields, run `cargo test -p assay-types`, observe snapshot failures, run `cargo insta review` (or `--force-update-snapshots`), and commit the new snapshots. New types (`OrchestratorMode`, `MeshConfig`, `GossipConfig`) each need their own snapshot test added to `schema_snapshots.rs` under `#[cfg(feature = "orchestrate")]`.

The dispatch routing currently lives in two places: `assay-cli/src/commands/run.rs` (the `needs_orchestration()` predicate + `execute_orchestrated()` routing) and `assay-mcp/src/server.rs` (the `orchestrate_run` tool). For S01, mode dispatch means: after loading the manifest, match `manifest.mode` and route to `run_orchestrated()` (Dag), `run_mesh()` (Mesh), or `run_gossip()` (Gossip). DAG mode preserves all existing logic; the stubs can return a completed `OrchestratorResult` with zero outcomes (or delegate to `run_orchestrated()` unchanged).

## Recommendation

Follow the exact pattern already used for `FailurePolicy` and `MergeStrategy` in this codebase: simple enum with `serde(rename_all = "snake_case")`, `Default` impl on `OrchestratorMode::Dag`, `serde(default)` on the `mode` field of `RunManifest`. For config structs (`MeshConfig`, `GossipConfig`), add them with `deny_unknown_fields` and `Option` fields with `serde(default)` defaults, matching the `ConflictResolutionConfig` pattern exactly.

For dispatch, add a `mode()` accessor or match directly on `manifest.mode` in the two call sites (CLI `execute()` and MCP `orchestrate_run()`). Keep the existing `needs_orchestration()` check intact for DAG mode — it still applies when mode is `Dag`. For `Mesh` and `Gossip`, always route to the new stub executors regardless of session count.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Schema snapshot testing | `insta` crate + `assert_json_snapshot!` macro | Already the pattern in `schema_snapshots.rs`; `cargo insta review` handles update flow |
| Atomic state persistence | `persist_state()` in `executor.rs` (NamedTempFile → rename) | Already handles sync ordering; copy the pattern for mesh/gossip stub state |
| Enum with snake_case serialization | `#[serde(rename_all = "snake_case")]` | Used on `FailurePolicy`, `SessionRunState`, etc. — direct copy |

## Existing Code and Patterns

- `crates/assay-types/src/manifest.rs` — `RunManifest` with `deny_unknown_fields`; adding `mode`, `mesh_config`, `gossip_config` here with `serde(default, skip_serializing_if = "…")`. **Critical**: `deny_unknown_fields` means all three new fields must be added before any existing JSON with unknown fields is tested — but since the fields have `serde(default)`, existing TOML without them continues to parse correctly.
- `crates/assay-types/src/orchestrate.rs` — `OrchestratorStatus`, `FailurePolicy`, `OrchestratorMode` and the new mesh/gossip types go here. Pattern: derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema` + `deny_unknown_fields` on structs, `inventory::submit!` for schema registry. Re-export new public types from `lib.rs` under the `#[cfg(feature = "orchestrate")]` block.
- `crates/assay-types/src/lib.rs` lines 64-68 — the `pub use orchestrate::{…}` re-export block under the `orchestrate` feature. New types (`OrchestratorMode`, `MeshConfig`, `GossipConfig`) must be added here.
- `crates/assay-core/src/orchestrate/mod.rs` — just `pub mod` declarations; add `pub mod mesh;` and `pub mod gossip;`.
- `crates/assay-core/src/orchestrate/executor.rs` — `OrchestratorConfig` and `run_orchestrated()` are the template for stub signatures. `run_mesh()` and `run_gossip()` stubs must accept `(manifest, config, pipeline_config, session_runner)` and return `Result<OrchestratorResult, AssayError>`.
- `crates/assay-cli/src/commands/run.rs` — `needs_orchestration()` predicate and `execute()` routing. S01 adds a mode match before the existing DAG path: if `manifest.mode == Mesh` → `execute_mesh()` stub, if `Gossip` → `execute_gossip()` stub, else fall through to existing `needs_orchestration()` logic. This is additive — no existing code changes except adding the match at the top of `execute()`.
- `crates/assay-mcp/src/server.rs` — `orchestrate_run` handler. After parsing params, match manifest mode and route. The existing multi-session check (`sessions.len() < 2 && !has_deps`) should only apply to DAG mode; Mesh/Gossip bypass it since they are always parallel.
- `crates/assay-types/tests/schema_snapshots.rs` — add snapshot tests for all new types under `#[cfg(feature = "orchestrate")]`, following the exact pattern of the 15+ existing orchestrate snapshot tests.

## Constraints

- `deny_unknown_fields` on `RunManifest`: adding `mode`, `mesh_config`, `gossip_config` must use `serde(default)` + `skip_serializing_if` so existing manifests without these fields deserialize correctly. Failing to add `skip_serializing_if` won't break deserialization but will make the run-manifest snapshot test regenerate unnecessarily on empty manifests.
- `OrchestratorStatus` has `deny_unknown_fields` — the D054 extension (`mesh_status`, `gossip_status`) is for S04, not S01. Do NOT touch `OrchestratorStatus` in this slice.
- The `orchestrate` feature gate in `assay-types`: all new orchestration types must be behind `#[cfg(feature = "orchestrate")]` (module declaration in `lib.rs` is already gated). The existing `orchestrate.rs` module is unconditional but only the pub re-exports are feature-gated — check `lib.rs` line 29 carefully: it reads `pub mod orchestrate;` unconditionally, then `#[cfg(feature = "orchestrate")] pub use orchestrate::{…}` — new types follow the same pattern.
- `MeshConfig` and `GossipConfig` need `deny_unknown_fields` as they will be persisted/serialized per D009 principles.
- Stub functions must be syntactically valid and compile with `just build` — they can return `Ok(OrchestratorResult { run_id: ulid, outcomes: vec![], duration: Duration::ZERO, failure_policy })` or delegate entirely to `run_orchestrated()` for now.
- `depends_on` warning: per D053, when mode is Mesh or Gossip, sessions with non-empty `depends_on` should emit `tracing::warn!`. This warning should be in the stub executor (it's the executor's responsibility, not the types layer).

## Common Pitfalls

- **`deny_unknown_fields` + `serde(default)` on `RunManifest`** — The `#[serde(deny_unknown_fields)]` on `RunManifest` means adding `mode` without `serde(default)` would break existing TOML files. Use `#[serde(default)]` on the `mode` field directly (not just `skip_serializing_if`). The `Default` impl on `OrchestratorMode` must return `Dag` for this to work.
- **Schema snapshot regeneration order** — Run `cargo test -p assay-types` first to see all failing snapshots, then `cargo insta review` to accept. Don't run `--force-update-snapshots` before inspecting the diffs — there may be unintended changes if the existing schema broke.
- **MCP `orchestrate_run` multi-session guard** — The current handler rejects manifests with fewer than 2 sessions and no deps for DAG mode, which is correct. When mode is Mesh or Gossip, this guard must not apply (a single-session mesh run is valid, if unusual). The guard should be conditioned on `manifest.mode == OrchestratorMode::Dag`.
- **`OrchestratorMode` in TOML vs JSON** — The mode field is TOML-authored (manifests) but also JSON-serialized (state files). `serde(rename_all = "snake_case")` ensures both `"dag"`, `"mesh"`, `"gossip"` work correctly in both formats.
- **Re-export placement** — `assay-types/src/lib.rs` already has `pub use orchestrate::{…}` under `#[cfg(feature = "orchestrate")]`. New types must be added to this `pub use` list — forgetting this breaks the public API even if the types compile fine internally.
- **`OrchestratorMode` needs `serde(default)` for the RunManifest field** — If TOML without `mode =` is loaded, `serde(default)` on the field triggers `OrchestratorMode::default()` which returns `Dag`. Without this, existing manifests would fail to deserialize.

## Open Risks

- **Snapshot diff scope**: Adding `mode`, `mesh_config`, `gossip_config` to `RunManifest` changes its JSON schema significantly (new optional properties). The `run-manifest-schema.snap` and `manifest-session-schema.snap` (if it exists) will both need regeneration. Verify the new schema diffs are exactly the expected additions before committing.
- **`OrchestratorMode` location**: The roadmap places it in `assay-types/src/orchestrate.rs` alongside the existing orchestration types. This is correct — but `RunManifest` in `assay-types/src/manifest.rs` will import from `crate::orchestrate::OrchestratorMode`. Since `orchestrate.rs` is already a module in `assay-types`, this is a within-crate import and works regardless of the feature gate (the type itself is always defined; only the re-export is gated). However, `RunManifest` itself is not behind a feature gate — it's unconditionally public. This means `OrchestratorMode` must also be unconditionally defined (not feature-gated) even if its re-export is. Confirm `orchestrate.rs` module is unconditional in `lib.rs` (it is — line 29).
- **CLI mode display**: The roadmap calls for mode display in CLI output in S04, not S01. S01 only needs dispatch routing, not display — avoid scope creep here.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / serde / schemars | none needed — patterns are well-established in codebase | none found |
| insta snapshot testing | none needed — pattern is already in use across 50+ snapshots | none found |

## Sources

- All findings from direct codebase inspection of `assay-types/src/manifest.rs`, `assay-types/src/orchestrate.rs`, `assay-types/src/lib.rs`, `assay-core/src/orchestrate/{mod,executor}.rs`, `assay-cli/src/commands/run.rs`, `assay-mcp/src/server.rs`, and `assay-types/tests/schema_snapshots.rs`.
