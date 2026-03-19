---
estimated_steps: 5
estimated_files: 3
---

# T01: Add OrchestratorMode, MeshConfig, GossipConfig to assay-types with snapshots

**Slice:** S01 — Mode Infrastructure
**Milestone:** M004

## Description

Add the three new coordination-mode types to `assay-types::orchestrate`: the `OrchestratorMode` enum (`Dag` | `Mesh` | `Gossip`), the `MeshConfig` struct (heartbeat and timeout configuration), and the `GossipConfig` struct (coordinator interval configuration). Re-export them from `lib.rs` under the existing `orchestrate` feature gate. Add snapshot tests for all three types and lock their schemas.

This is purely additive — no existing types or fields are modified. The types must be unconditionally defined in `orchestrate.rs` (the module is unconditionally declared in `lib.rs` line 29); only the re-exports are feature-gated.

## Steps

1. In `crates/assay-types/src/orchestrate.rs`, add `OrchestratorMode` enum with `#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]`, `#[serde(rename_all = "snake_case")]`. Variants: `#[default] Dag`, `Mesh`, `Gossip`. Add `inventory::submit!` for schema registry.

2. Add `MeshConfig` struct with `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`. Fields: `heartbeat_interval_secs: u64` (serde default fn returning 5), `suspect_timeout_secs: u64` (default 10), `dead_timeout_secs: u64` (default 30). Implement `Default`. Add `inventory::submit!`.

3. Add `GossipConfig` struct with `deny_unknown_fields`. Fields: `coordinator_interval_secs: u64` (serde default fn returning 5). Implement `Default`. Add `inventory::submit!`.

4. In `crates/assay-types/src/lib.rs`, add `OrchestratorMode`, `MeshConfig`, `GossipConfig` to the `#[cfg(feature = "orchestrate")] pub use orchestrate::{...}` block.

5. In `crates/assay-types/tests/schema_snapshots.rs`, add three snapshot tests under `#[cfg(feature = "orchestrate")]`:
   ```rust
   #[cfg(feature = "orchestrate")] #[test] fn orchestrator_mode_schema_snapshot() { ... }
   #[cfg(feature = "orchestrate")] #[test] fn mesh_config_schema_snapshot() { ... }
   #[cfg(feature = "orchestrate")] #[test] fn gossip_config_schema_snapshot() { ... }
   ```
   Run `cargo test -p assay-types --features orchestrate 2>&1 | grep FAILED` to see snapshot failures, then `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` to generate them (or `cargo insta review` if interactive). Verify the diff looks correct before committing.

## Must-Haves

- [ ] `OrchestratorMode::Dag` is the `#[default]` variant; `OrchestratorMode::default()` returns `Dag`
- [ ] `serde(rename_all = "snake_case")` on `OrchestratorMode` — `"dag"`, `"mesh"`, `"gossip"` in JSON/TOML
- [ ] `MeshConfig` and `GossipConfig` have `deny_unknown_fields`
- [ ] `MeshConfig::default()` returns sensible values (heartbeat 5s, suspect 10s, dead 30s)
- [ ] `GossipConfig::default()` returns sensible values (coordinator 5s)
- [ ] All three types have `inventory::submit!` schema registry entries
- [ ] All three are re-exported from `assay-types` under `#[cfg(feature = "orchestrate")]`
- [ ] Three new `.snap` files created in `crates/assay-types/tests/snapshots/`
- [ ] Unit tests in `orchestrate.rs` cover: serde round-trip for all `OrchestratorMode` variants, `default()` returns `Dag`, `deny_unknown_fields` rejection for `MeshConfig` and `GossipConfig`
- [ ] `cargo test -p assay-types --features orchestrate` green with 0 failures

## Verification

- `cargo test -p assay-types --features orchestrate` — all tests pass
- `ls crates/assay-types/tests/snapshots/ | grep -E "orchestrator-mode|mesh-config|gossip-config"` — shows 3 new snap files
- `cargo clippy -p assay-types --features orchestrate` — 0 warnings on new code

## Observability Impact

- Signals added/changed: None (pure type definitions; no runtime behavior)
- How a future agent inspects this: Schema snapshots in `crates/assay-types/tests/snapshots/` are the canonical locked contract; `cargo test -p assay-types --features orchestrate` verifies schema stability
- Failure state exposed: `serde` deserialization errors for unknown fields on `MeshConfig`/`GossipConfig` are immediate with field name

## Inputs

- `crates/assay-types/src/orchestrate.rs` — existing orchestration types to follow as pattern (`FailurePolicy`, `ConflictResolutionConfig`)
- `crates/assay-types/src/lib.rs` — existing `pub use orchestrate::{...}` block for re-export placement
- `crates/assay-types/tests/schema_snapshots.rs` — existing snapshot test pattern under `#[cfg(feature = "orchestrate")]`

## Expected Output

- `crates/assay-types/src/orchestrate.rs` — `OrchestratorMode`, `MeshConfig`, `GossipConfig` added with full derives, defaults, registry entries, and unit tests
- `crates/assay-types/src/lib.rs` — three new types added to the `pub use orchestrate::{...}` block
- `crates/assay-types/tests/schema_snapshots.rs` — three new snapshot test functions
- `crates/assay-types/tests/snapshots/orchestrator-mode-schema.snap` (new)
- `crates/assay-types/tests/snapshots/mesh-config-schema.snap` (new)
- `crates/assay-types/tests/snapshots/gossip-config-schema.snap` (new)
