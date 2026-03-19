---
id: S01
parent: M004
milestone: M004
provides:
  - OrchestratorMode enum (Dag | Mesh | Gossip) with Default=Dag and snake_case serde — locked schema snapshot
  - MeshConfig struct (heartbeat_interval_secs, suspect_timeout_secs, dead_timeout_secs) with deny_unknown_fields — locked schema snapshot
  - GossipConfig struct (coordinator_interval_secs) with deny_unknown_fields — locked schema snapshot
  - RunManifest extended with mode/mesh_config/gossip_config fields and backward-compatible serde defaults — updated schema snapshot
  - run_mesh() stub and run_gossip() stub in assay-core::orchestrate with tracing::warn for depends_on sessions
  - CLI execute() mode dispatch: Mesh/Gossip route to stubs and return early before needs_orchestration check
  - MCP orchestrate_run multi-session guard conditioned on mode == Dag; Mesh/Gossip bypass it and route to stubs
  - impl Default for RunManifest (Dag, empty sessions) to unblock test struct literals across the workspace
requires: []
affects:
  - slice: S02
    provides: OrchestratorMode::Mesh dispatch routing, MeshConfig type, run_mesh() stub (replaced by full implementation)
  - slice: S03
    provides: OrchestratorMode::Gossip dispatch routing, GossipConfig type, run_gossip() stub (replaced by full implementation)
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/src/manifest.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-mode-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-config-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__gossip-config-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/src/orchestrate/gossip.rs
  - crates/assay-core/src/orchestrate/mod.rs
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - D052: Mode dispatch via free functions (run_mesh, run_gossip) — zero-trait convention
  - D053: Mesh/Gossip modes ignore depends_on with tracing::warn — mode is exclusive
  - D055: MeshConfig/GossipConfig as optional top-level RunManifest fields — flat manifest, no polymorphic union
  - D056: impl Default for RunManifest instead of cascading struct-literal updates — serde contract unchanged
patterns_established:
  - Mode dispatch in CLI: match on manifest.mode before needs_orchestration(); Dag falls through, Mesh/Gossip return early
  - MCP mode routing: match arm before the DAG spawn_blocking block; each mode calls stub via spawn_blocking and returns early
  - Stub executor signature: run_mesh/run_gossip accept (&RunManifest, &OrchestratorConfig, &PipelineConfig, &F) matching run_orchestrated
  - New coordination-mode types: enum derives (Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema); structs use deny_unknown_fields with serde default fns
observability_surfaces:
  - tracing::warn! per session with non-empty depends_on when mode is Mesh or Gossip — observable via RUST_LOG=warn
  - Schema snapshots in crates/assay-types/tests/snapshots/ are the canonical locked contract
  - cargo test -p assay-types --features orchestrate verifies schema stability across changes
drill_down_paths:
  - .kata/milestones/M004/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M004/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M004/slices/S01/tasks/T03-SUMMARY.md
duration: ~2.5h (T01: short, T02: ~15m, T03: ~1h)
verification_result: passed
completed_at: 2026-03-17
---

# S01: Mode Infrastructure

**`OrchestratorMode` enum, `MeshConfig`, and `GossipConfig` added to `assay-types` with locked schema snapshots; `RunManifest` extended with backward-compatible mode fields; `run_mesh()`/`run_gossip()` stubs wired through CLI and MCP dispatch; `just ready` green with 0 warnings across all 1222+ tests.**

## What Happened

**T01** added `OrchestratorMode` (`Dag` | `Mesh` | `Gossip`), `MeshConfig`, and `GossipConfig` to `assay-types::orchestrate`. All three have full derives, `serde(rename_all = "snake_case")` for the enum, `deny_unknown_fields` and `Default` impls for the structs, `inventory::submit!` schema registry entries, and unit tests covering round-trips and field defaults. Three new snapshot files were generated and accepted.

**T02** extended `RunManifest` with `mode: OrchestratorMode` (serde default = `Dag`), `mesh_config: Option<MeshConfig>`, and `gossip_config: Option<GossipConfig>`. Both config fields use `skip_serializing_if = "Option::is_none"`. The `run-manifest-schema.snap` was regenerated — the diff was purely additive (three new optional properties). Backward compatibility was confirmed by a unit test: a TOML with only `[[sessions]]\nspec = "auth"` deserializes to `mode: Dag, mesh_config: None, gossip_config: None`.

**T03** created `mesh.rs` and `gossip.rs` stubs under `assay-core::orchestrate`. Both accept the same four-argument signature as `run_orchestrated()`, emit `tracing::warn!` per session with non-empty `depends_on`, and return a valid `OrchestratorResult` with a fresh ULID run_id, zero outcomes, and `Duration::ZERO`. Both modules are declared in `mod.rs`.

CLI `execute()` gained a `match manifest.mode { Mesh => ..., Gossip => ..., Dag => {} }` block before the `needs_orchestration()` check. MCP `orchestrate_run` had its multi-session guard conditioned on `mode == Dag`; Mesh and Gossip arms bypass it and route to stubs via `spawn_blocking`.

A cascade of struct-literal test failures across the workspace (manifest.rs, dag.rs, executor.rs, pipeline.rs, orchestrate_integration.rs, run.rs) was resolved by adding `impl Default for RunManifest` — explicit, not derived, so no schema change. This unblocked all downstream tests without requiring individual struct-literal edits.

## Verification

```
cargo test -p assay-types --features orchestrate  → 58 passed
cargo test -p assay-core --features orchestrate   → 5 passed (unit + integration)
cargo test -p assay-cli                           → 30 passed (3 new mode dispatch tests)
cargo test -p assay-mcp                           → 112 + 29 passed (2 new guard-bypass tests)
just ready                                        → fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓
```

Schema snapshot grep confirms 3 new `.snap` files (`orchestrator-mode-schema`, `mesh-config-schema`, `gossip-config-schema`) and an updated `run-manifest-schema.snap`.

## Requirements Advanced

- R034 (OrchestratorMode selection) — `mode` field on `RunManifest` now parses `dag`/`mesh`/`gossip`; dispatch routing in CLI and MCP routes to correct executor entry points; schema snapshot locked and backward-compatible

## Requirements Validated

- R034 — Fully validated: `OrchestratorMode` enum exists with schema snapshot, `mode` field deserializes correctly (missing field → Dag, `mode = "mesh"` → Mesh, `mode = "gossip"` → Gossip), dispatch routing exercised by unit tests in CLI (3 tests) and MCP (2 tests), existing 1222+ tests pass, `just ready` green

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- **`impl Default for RunManifest`** — not in the task plan. T02 added `deny_unknown_fields` fields to `RunManifest` without updating all test struct literals in the workspace. Adding `Default` (D056) resolved the cascade cleanly without changing the serde contract. Snapshot diff confirmed no schema change from this addition.
- **Fixed `orchestrate_integration.rs` make_manifest helper** — the T02 plan did not identify this test helper as needing updates. Caught during T03 test compilation.
- **Fixed `run_manifest_with_scoped_sessions_validates` in `schema_roundtrip.rs`** — identified during T02 snapshot acceptance; test constructed a `RunManifest` literal and required the three new fields.

## Known Limitations

- Stubs return zero outcomes and do not exercise the `session_runner` closure (it's `unreachable!`). This is intentional — full implementations come in S02 (Mesh) and S03 (Gossip).
- Mesh and Gossip modes have no integration test coverage yet. The MCP guard-bypass and CLI mode dispatch tests are unit-level only.
- `MeshConfig` and `GossipConfig` fields on `RunManifest` are accepted but silently ignored if the mode doesn't match. No cross-field validation (e.g., warning when `gossip_config` is set but `mode = "mesh"`) — deferred.

## Follow-ups

- S02: Replace `run_mesh()` stub with full implementation (parallel dispatch, roster injection, inbox/outbox directories, routing thread, SWIM membership)
- S03: Replace `run_gossip()` stub with full implementation (coordinator thread, knowledge manifest, manifest path injection)
- Future: Add validation warning when `mesh_config`/`gossip_config` is set but mode doesn't match

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — Added OrchestratorMode, MeshConfig, GossipConfig with derives, inventory entries, and 9 unit tests
- `crates/assay-types/src/lib.rs` — Added OrchestratorMode, MeshConfig, GossipConfig to orchestrate feature-gated pub use block
- `crates/assay-types/src/manifest.rs` — Added import, three new fields on RunManifest, 5 unit tests, impl Default for RunManifest, fixed one test literal
- `crates/assay-types/tests/schema_snapshots.rs` — Added orchestrator_mode, mesh_config, gossip_config snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-mode-schema.snap` — New locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-config-schema.snap` — New locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-config-schema.snap` — New locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — Updated with 3 new optional properties
- `crates/assay-types/tests/schema_roundtrip.rs` — Fixed RunManifest literal in run_manifest_with_scoped_sessions_validates
- `crates/assay-core/src/orchestrate/mesh.rs` — New: run_mesh() stub with warn loop and 2 unit tests
- `crates/assay-core/src/orchestrate/gossip.rs` — New: run_gossip() stub with warn loop and 2 unit tests
- `crates/assay-core/src/orchestrate/mod.rs` — Added pub mod mesh; pub mod gossip;
- `crates/assay-core/src/orchestrate/dag.rs` — Fixed test make_manifest helper (..Default::default())
- `crates/assay-core/src/orchestrate/executor.rs` — Fixed test make_manifest helper
- `crates/assay-core/src/manifest.rs` — Fixed test struct literals (9 occurrences)
- `crates/assay-core/src/pipeline.rs` — Fixed test RunManifest literal
- `crates/assay-core/tests/orchestrate_integration.rs` — Fixed make_manifest helper
- `crates/assay-cli/src/commands/run.rs` — Mode dispatch in execute(), execute_mesh/gossip stubs, 3 unit tests
- `crates/assay-mcp/src/server.rs` — OrchestratorMode import, guard conditioned on Dag, Mesh/Gossip routing, 2 unit tests

## Forward Intelligence

### What the next slice should know
- The stub signatures (`run_mesh` / `run_gossip`) match `run_orchestrated()` exactly — `(&RunManifest, &OrchestratorConfig, &PipelineConfig, &F)`. S02 replaces the stub body; the signature stays.
- `OrchestratorConfig` carries `failure_policy` and `max_concurrency` — Mesh mode should respect `max_concurrency` for parallel session launch even without dependency ordering.
- `impl Default for RunManifest` is in `manifest.rs` (not derived). Any new `deny_unknown_fields` field on `RunManifest` needs a matching update in that `Default` impl — otherwise tests that use `..Default::default()` will fail to compile.
- The MCP guard bypass for Mesh/Gossip is in `orchestrate_run`. The stub is called via `spawn_blocking`. When S02 replaces the stub with a real implementation, the `spawn_blocking` wrapper stays — no call-site changes needed.

### What's fragile
- `Default::default()` for `RunManifest` — callers using `..Default::default()` in struct literals will silently get the stub defaults if new fields are added without updating the `Default` impl. This is compile-safe only if the new field has no `Default` impl itself.
- The MCP `orchestrate_run` Mesh/Gossip routing arms are minimal stubs — they return `OrchestrateRunResponse { run_id, sessions: vec![], ... }` with no real state persisted. `orchestrate_status` on these run IDs will return not-found until S02/S03 write real state.

### Authoritative diagnostics
- `cargo test -p assay-types --features orchestrate` — most reliable first check; catches schema drift before it cascades
- `crates/assay-types/tests/snapshots/` — diff these to confirm any RunManifest or orchestrate-type change is additive-only
- `RUST_LOG=warn cargo test` — surfaces tracing::warn! messages from Mesh/Gossip stub execution

### What assumptions changed
- Original assumption: T02 adding new fields would only affect `manifest.rs` and `schema_snapshots.rs`. Actual: adding fields to a `deny_unknown_fields` struct cascades to all struct-literal test constructions across the workspace. `impl Default` is the right mitigation pattern for this class of problem.
