# S01: Mode Infrastructure

**Goal:** `OrchestratorMode` enum, `MeshConfig`, and `GossipConfig` types exist in `assay-types`; `RunManifest` has `mode`, `mesh_config`, `gossip_config` fields with `serde(default)`; dispatch routing in CLI and MCP routes to `run_mesh()`/`run_gossip()` stubs; all schema snapshots updated and locked; `just ready` green.
**Demo:** A manifest TOML with `mode = "mesh"` parses without error and routes to the `run_mesh()` stub (returns empty `OrchestratorResult`); a manifest with no `mode` field routes to the existing DAG executor unchanged; all 1222+ existing tests pass; `just ready` outputs no warnings or errors.

## Must-Haves

- `OrchestratorMode` enum (`Dag`, `Mesh`, `Gossip`) in `assay-types::orchestrate` with `Default = Dag`, `serde(rename_all = "snake_case")`, `JsonSchema`, schema snapshot locked
- `MeshConfig` struct (`heartbeat_interval_secs`, `suspect_timeout_secs`, `dead_timeout_secs`) in `assay-types::orchestrate` with `deny_unknown_fields`, schema snapshot locked
- `GossipConfig` struct (`coordinator_interval_secs`) in `assay-types::orchestrate` with `deny_unknown_fields`, schema snapshot locked
- `mode: OrchestratorMode` field on `RunManifest` with `#[serde(default)]`; existing TOML without `mode` still deserializes to `Dag`
- `mesh_config: Option<MeshConfig>` and `gossip_config: Option<GossipConfig>` on `RunManifest` with `serde(default, skip_serializing_if = "Option::is_none")`
- `run-manifest-schema.snap` regenerated and committed with the three new optional properties
- `run_mesh()` stub and `run_gossip()` stub in `assay-core::orchestrate::mesh` and `::gossip` — compile, accept `(manifest, config, pipeline_config, session_runner)`, emit `tracing::warn!` per session with non-empty `depends_on`, return `Ok(OrchestratorResult { ... })` with zero outcomes
- CLI `execute()` matches `manifest.mode` before the existing `needs_orchestration()` check: `Mesh` → `run_mesh()` stub, `Gossip` → `run_gossip()` stub, `Dag` → existing path
- MCP `orchestrate_run` multi-session guard conditioned on `mode == Dag`; `Mesh` and `Gossip` bypass it and route to stubs
- `just ready` passes (fmt ✓, lint ✓, test ✓, deny ✓)

## Proof Level

- This slice proves: contract + operational
- Real runtime required: no (stub executors, no real agent launch)
- Human/UAT required: no

## Verification

- `cargo test -p assay-types --features orchestrate` — all existing + new snapshot tests pass (including `orchestrator-mode-schema`, `mesh-config-schema`, `gossip-config-schema`, updated `run-manifest-schema`)
- `cargo test -p assay-core --features orchestrate` — mesh/gossip stub modules compile and their unit tests pass
- `cargo test -p assay-cli` — existing CLI tests pass; mode-dispatch unit tests pass
- `cargo test -p assay-mcp` — existing MCP tests pass; single-session mesh/gossip guard-bypass test passes
- `just ready` — fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` emitted per manifest session with non-empty `depends_on` when mode is `Mesh` or `Gossip` — observable in test output and runtime logs
- Inspection surfaces: schema snapshots in `crates/assay-types/tests/snapshots/` are the canonical locked contract; diff them to verify additive-only changes
- Failure visibility: `serde` deserialization errors on `RunManifest` are immediate and include the offending field (existing behavior preserved)
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `assay-types::orchestrate` (existing), `assay-types::manifest` (existing), `assay-core::orchestrate::executor` (for `OrchestratorConfig`/`OrchestratorResult` signature reference), CLI `execute()` routing, MCP `orchestrate_run` handler
- New wiring introduced in this slice: `OrchestratorMode` import in `manifest.rs`; `pub mod mesh; pub mod gossip;` in `orchestrate/mod.rs`; `match manifest.mode { ... }` dispatch at both CLI and MCP call sites
- What remains before the milestone is truly usable end-to-end: S02 (run_mesh full implementation with routing thread, roster injection, SWIM membership) and S03 (run_gossip full implementation with knowledge manifest)

## Tasks

- [x] **T01: Add OrchestratorMode, MeshConfig, GossipConfig to assay-types with snapshots** `est:45m`
  - Why: These are the foundational types for mode selection and per-mode configuration. All downstream work depends on them being schema-locked.
  - Files: `crates/assay-types/src/orchestrate.rs`, `crates/assay-types/src/lib.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do: In `orchestrate.rs`, add `OrchestratorMode` enum (`Dag`, `Mesh`, `Gossip`) with `#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]`, `#[serde(rename_all = "snake_case")]`, `#[default]` on `Dag`. Add `MeshConfig` struct with `heartbeat_interval_secs: u64` (default 5), `suspect_timeout_secs: u64` (default 10), `dead_timeout_secs: u64` (default 30), `deny_unknown_fields`. Add `GossipConfig` struct with `coordinator_interval_secs: u64` (default 5), `deny_unknown_fields`. Add `inventory::submit!` registry entries for all three. Add unit tests (serde round-trip, default values, snake_case serialization, deny_unknown_fields). Add `Default` impls for `MeshConfig` and `GossipConfig`. In `lib.rs`, add `OrchestratorMode`, `MeshConfig`, `GossipConfig` to the `#[cfg(feature = "orchestrate")] pub use orchestrate::{...}` block. In `schema_snapshots.rs`, add `orchestrator_mode_schema_snapshot`, `mesh_config_schema_snapshot`, `gossip_config_schema_snapshot` tests under `#[cfg(feature = "orchestrate")]`. Run `cargo test -p assay-types --features orchestrate` to generate snapshots, then `cargo insta review` (or `--force-update-snapshots`) to accept them.
  - Verify: `cargo test -p assay-types --features orchestrate` — all tests pass including the 3 new snapshot tests; `grep -r "orchestrator-mode-schema" crates/assay-types/tests/snapshots/` finds a `.snap` file
  - Done when: 3 new snapshot files exist, `cargo test -p assay-types --features orchestrate` is green, no clippy warnings on the new types

- [x] **T02: Add mode/mesh_config/gossip_config fields to RunManifest and regenerate manifest snapshot** `est:30m`
  - Why: `RunManifest` is the user-facing TOML contract. Adding the mode fields with `serde(default)` is the change that makes `mode = "mesh"` parse correctly while keeping existing manifests backward-compatible.
  - Files: `crates/assay-types/src/manifest.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do: In `manifest.rs`, add `use crate::orchestrate::{GossipConfig, MeshConfig, OrchestratorMode};` (within-crate import; works regardless of feature gate since `orchestrate.rs` module is unconditionally declared). Add three fields to `RunManifest`: `#[serde(default)] pub mode: OrchestratorMode`, `#[serde(default, skip_serializing_if = "Option::is_none")] pub mesh_config: Option<MeshConfig>`, `#[serde(default, skip_serializing_if = "Option::is_none")] pub gossip_config: Option<GossipConfig>`. Add unit tests: (1) manifest without `mode` field deserializes to `Dag`; (2) `mode = "mesh"` deserializes to `Mesh`; (3) `mode = "gossip"` with `[mesh_config]` is silently present but ignored; (4) manifest with `mode` serializes and omits `mesh_config`/`gossip_config` when absent. Run `cargo test -p assay-types --features orchestrate` — `run-manifest-schema.snap` will fail; run `cargo insta review` to accept the updated snapshot (verify the diff only adds the three new optional properties and nothing else changes).
  - Verify: `cargo test -p assay-types --features orchestrate` green; TOML `[[sessions]]\nspec = "auth"` without `mode` parses to `RunManifest { mode: Dag, mesh_config: None, ... }` in a unit test; snapshot diff shows only the expected 3 new properties
  - Done when: `run-manifest-schema.snap` updated and accepted, all manifest tests pass, backward-compat test covers missing `mode` field

- [x] **T03: Add run_mesh/run_gossip stubs, wire mod.rs, dispatch in CLI and MCP** `est:45m`
  - Why: Without dispatch routing, the new types are inert. This task closes the slice by making `mode = "mesh"` and `mode = "gossip"` actually route to real (stub) functions, completing R034.
  - Files: `crates/assay-core/src/orchestrate/mod.rs`, `crates/assay-core/src/orchestrate/mesh.rs` (new), `crates/assay-core/src/orchestrate/gossip.rs` (new), `crates/assay-cli/src/commands/run.rs`, `crates/assay-mcp/src/server.rs`
  - Do: Create `mesh.rs` with `pub fn run_mesh<F>(manifest, config, pipeline_config, session_runner: &F) -> Result<OrchestratorResult, AssayError>` — emit `tracing::warn!` for each session with `!depends_on.is_empty()`, return `Ok(OrchestratorResult { run_id: Ulid::new().to_string(), outcomes: vec![], duration: Duration::ZERO, failure_policy: config.failure_policy })`. Create `gossip.rs` with the same signature pattern. Add `pub mod mesh; pub mod gossip;` to `mod.rs`. In CLI `run.rs`, in `execute()` before the `needs_orchestration()` call, add `match manifest.mode { OrchestratorMode::Mesh => execute_mesh(cmd, &manifest, &pipeline_config), OrchestratorMode::Gossip => execute_gossip(cmd, &manifest, &pipeline_config), OrchestratorMode::Dag => { /* fall through */ } }`. Add `execute_mesh()` and `execute_gossip()` CLI handlers that call the stubs and return a minimal `OrchestrationResponse`. In MCP `orchestrate_run`, change the multi-session guard to `if manifest.mode == OrchestratorMode::Dag && manifest.sessions.len() < 2 && !has_deps { ... }`. Add import for `OrchestratorMode` in both CLI and MCP. Add unit tests for mode dispatch routing in CLI and the guard-bypass in MCP. Run `just build` first, then `just test`, then `just ready`.
  - Verify: `just ready` green (fmt + lint + test + deny); `cargo test -p assay-cli` includes a test that a single-session Mesh manifest bypasses `needs_orchestration`; `cargo test -p assay-mcp` includes a test that `orchestrate_run` with `mode = "mesh"` skips the multi-session guard
  - Done when: `just ready` passes with 0 warnings, dispatch routing is exercised by at least one unit test per call site, stubs compile and are callable

## Files Likely Touched

- `crates/assay-types/src/orchestrate.rs`
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/src/manifest.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-types/tests/snapshots/` (new/updated `.snap` files)
- `crates/assay-core/src/orchestrate/mod.rs`
- `crates/assay-core/src/orchestrate/mesh.rs` (new)
- `crates/assay-core/src/orchestrate/gossip.rs` (new)
- `crates/assay-cli/src/commands/run.rs`
- `crates/assay-mcp/src/server.rs`
