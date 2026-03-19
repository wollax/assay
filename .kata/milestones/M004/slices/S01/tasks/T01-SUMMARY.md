---
id: T01
parent: S01
milestone: M004
provides:
  - OrchestratorMode enum (Dag | Mesh | Gossip) with Default=Dag and snake_case serde
  - MeshConfig struct with heartbeat/suspect/dead timeout defaults and deny_unknown_fields
  - GossipConfig struct with coordinator interval default and deny_unknown_fields
  - All three types re-exported from assay-types under the orchestrate feature gate
  - Schema registry inventory::submit! entries for all three types
  - Locked snapshot files for all three schemas
  - Unit tests covering serde round-trips, defaults, and deny_unknown_fields
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-mode-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-config-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__gossip-config-schema.snap
key_decisions:
  - none (pure additive, followed all existing patterns in orchestrate.rs)
patterns_established:
  - New coordination-mode types follow same derive stack as FailurePolicy (Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema) for enums; structs use deny_unknown_fields with serde default fns
observability_surfaces:
  - Schema snapshots in crates/assay-types/tests/snapshots/ are the canonical locked contract — diff them to verify additive-only changes
  - cargo test -p assay-types --features orchestrate verifies schema stability
duration: short
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add OrchestratorMode, MeshConfig, GossipConfig to assay-types with snapshots

**Added three coordination-mode types (`OrchestratorMode`, `MeshConfig`, `GossipConfig`) to `assay-types::orchestrate` with full derives, defaults, deny_unknown_fields, schema registry entries, unit tests, and locked snapshot files.**

## What Happened

Added `OrchestratorMode` enum (`Dag` | `Mesh` | `Gossip`) with `#[default] Dag` and `serde(rename_all = "snake_case")` serializing to `"dag"`, `"mesh"`, `"gossip"`. Added `MeshConfig` struct with `heartbeat_interval_secs` (default 5), `suspect_timeout_secs` (default 10), `dead_timeout_secs` (default 30), and `deny_unknown_fields`. Added `GossipConfig` struct with `coordinator_interval_secs` (default 5) and `deny_unknown_fields`. Both structs implement `Default` via explicit `impl` using the serde default fns.

All three types have `inventory::submit!` schema registry entries. All three are re-exported from `lib.rs` under `#[cfg(feature = "orchestrate")]`. Unit tests cover: serde round-trips for all OrchestratorMode variants, snake_case serialization values, `default()` returns `Dag`, `MeshConfig::default()` field values, `GossipConfig::default()` field values, and `deny_unknown_fields` rejection for both structs.

Ran `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` to generate the three new snapshot files. All 58 tests passed.

## Verification

- `cargo test -p assay-types --features orchestrate` → **58 passed, 0 failed**
- `ls crates/assay-types/tests/snapshots/ | grep -E "orchestrator-mode|mesh-config|gossip-config"` → 3 files found
- `cargo clippy -p assay-types --features orchestrate` → 0 warnings on new code

## Diagnostics

- Schema snapshots in `crates/assay-types/tests/snapshots/` are the canonical locked contract
- `serde` deserialization errors for unknown fields on `MeshConfig`/`GossipConfig` are immediate with field name in the error message

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — Added OrchestratorMode enum, MeshConfig struct, GossipConfig struct, 3 inventory::submit! entries, and 9 new unit tests
- `crates/assay-types/src/lib.rs` — Added OrchestratorMode, MeshConfig, GossipConfig to the orchestrate feature-gated pub use block
- `crates/assay-types/tests/schema_snapshots.rs` — Added orchestrator_mode_schema_snapshot, mesh_config_schema_snapshot, gossip_config_schema_snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-mode-schema.snap` — New locked schema snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-config-schema.snap` — New locked schema snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-config-schema.snap` — New locked schema snapshot
