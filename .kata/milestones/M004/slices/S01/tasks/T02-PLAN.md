---
estimated_steps: 4
estimated_files: 3
---

# T02: Add mode/mesh_config/gossip_config fields to RunManifest and regenerate manifest snapshot

**Slice:** S01 — Mode Infrastructure
**Milestone:** M004

## Description

Extend `RunManifest` with three new optional fields: `mode`, `mesh_config`, and `gossip_config`. All three must use `serde(default)` so that existing TOML manifests without these fields continue to deserialize correctly — `deny_unknown_fields` on `RunManifest` means the fields must exist before any round-trip test with them can pass. The `run-manifest-schema.snap` snapshot will break and must be regenerated.

**Critical constraint**: `RunManifest` has `#[serde(deny_unknown_fields)]`. The `mode` field needs `#[serde(default)]` directly on the field (not just on the type) so TOML without `mode =` triggers `OrchestratorMode::default()` which returns `Dag`.

## Steps

1. In `crates/assay-types/src/manifest.rs`, add the import `use crate::orchestrate::{GossipConfig, MeshConfig, OrchestratorMode};`. This is a within-crate import; `orchestrate.rs` is unconditionally declared in `lib.rs` (line 29), so no feature gate is needed here.

2. Add three fields to `RunManifest` (after `sessions`):
   ```rust
   /// Coordination mode for this run. Defaults to `dag` (existing behavior).
   #[serde(default)]
   pub mode: OrchestratorMode,

   /// Mesh mode configuration. Ignored unless `mode = "mesh"`.
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub mesh_config: Option<MeshConfig>,

   /// Gossip mode configuration. Ignored unless `mode = "gossip"`.
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub gossip_config: Option<GossipConfig>,
   ```

3. Add unit tests in `manifest.rs`:
   - `manifest_without_mode_defaults_to_dag`: TOML with `[[sessions]]\nspec = "auth"` — deserializes to `RunManifest { mode: OrchestratorMode::Dag, mesh_config: None, gossip_config: None, ... }`
   - `manifest_with_mode_mesh_parses`: TOML with `mode = "mesh"\n[[sessions]]\nspec = "auth"` — deserializes to `mode: Mesh`
   - `manifest_with_mode_gossip_parses`: similar for `"gossip"` → `Gossip`
   - `manifest_mode_omitted_in_serialization_when_dag`: serialize `RunManifest` with `mode: Dag`, verify TOML output does NOT contain `mode =` (because `Dag` is the default — but note: `serde` doesn't `skip_serializing_if` for `mode`, it will serialize it. Actually since we don't use `skip_serializing_if` on mode, it WILL serialize. That's fine for the snapshot, just verify round-trip.)
   - `manifest_mesh_config_omitted_when_none`: serialize with `mode: Mesh, mesh_config: None` — TOML output doesn't contain `mesh_config`

4. Run `cargo test -p assay-types --features orchestrate` — `run-manifest-schema.snap` will fail. Inspect the diff to verify it only adds the three new optional properties. Accept with `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` or `cargo insta review`.

## Must-Haves

- [ ] `mode` field uses `#[serde(default)]` so existing manifests without it deserialize to `Dag`
- [ ] `mesh_config` and `gossip_config` use `#[serde(default, skip_serializing_if = "Option::is_none")]`
- [ ] Existing TOML round-trip tests in `manifest.rs` all still pass (backward compatibility)
- [ ] New backward-compat unit test proves a manifest without `mode =` deserializes to `OrchestratorMode::Dag`
- [ ] `run-manifest-schema.snap` regenerated and accepted; diff shows only 3 new optional properties added to the schema
- [ ] `cargo test -p assay-types --features orchestrate` green

## Verification

- `cargo test -p assay-types --features orchestrate` — all tests pass
- `cat crates/assay-types/tests/snapshots/run-manifest-schema.snap | grep -c "mode\|mesh_config\|gossip_config"` — confirms new properties are in the snapshot
- Manual check: TOML `[[sessions]]\nspec = "x"` (no `mode`) round-trips to `mode: Dag` in a unit test

## Observability Impact

- Signals added/changed: `RunManifest` deserialization errors now mention `mode`, `mesh_config`, `gossip_config` if mistyped
- How a future agent inspects this: Schema snapshot is the locked contract; unit tests verify backward-compat for TOML without the new fields
- Failure state exposed: `serde` immediately reports unknown field names — existing `deny_unknown_fields` behavior unchanged

## Inputs

- T01 output: `OrchestratorMode`, `MeshConfig`, `GossipConfig` types available in `crates/assay-types/src/orchestrate.rs`
- `crates/assay-types/src/manifest.rs` — existing `RunManifest` with `deny_unknown_fields` and existing unit tests
- `crates/assay-types/tests/snapshots/run-manifest-schema.snap` — will be updated by this task

## Expected Output

- `crates/assay-types/src/manifest.rs` — `RunManifest` extended with three new fields; 4-5 new unit tests
- `crates/assay-types/tests/snapshots/run-manifest-schema.snap` — updated to include `mode`, `mesh_config`, `gossip_config` in the JSON Schema
