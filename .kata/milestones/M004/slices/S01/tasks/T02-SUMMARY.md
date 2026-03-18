---
id: T02
parent: S01
milestone: M004
provides:
  - RunManifest with mode/mesh_config/gossip_config fields and backward-compatible serde defaults
  - Updated run-manifest-schema.snap with three new optional properties
key_files:
  - crates/assay-types/src/manifest.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap
  - crates/assay-types/tests/schema_roundtrip.rs
key_decisions:
  - none (followed existing pattern for optional serde fields with defaults)
patterns_established:
  - Fields added to deny_unknown_fields structs require explicit initialization in all existing struct-literal tests (fixed one in schema_roundtrip.rs)
observability_surfaces:
  - serde deserialization errors on RunManifest now mention mode/mesh_config/gossip_config if mistyped
  - run-manifest-schema.snap is the locked schema contract; diff it to verify additive-only changes
duration: ~15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Add mode/mesh_config/gossip_config fields to RunManifest and regenerate manifest snapshot

**Extended `RunManifest` with three mode fields (`mode`, `mesh_config`, `gossip_config`) behind `serde(default)`, added 5 backward-compat unit tests, fixed one test in `schema_roundtrip.rs`, and accepted the updated `run-manifest-schema.snap`.**

## What Happened

1. Added `use crate::orchestrate::{GossipConfig, MeshConfig, OrchestratorMode};` import to `manifest.rs`.
2. Added three fields to `RunManifest` after `sessions`:
   - `#[serde(default)] pub mode: OrchestratorMode` — defaults to `Dag` on missing TOML key
   - `#[serde(default, skip_serializing_if = "Option::is_none")] pub mesh_config: Option<MeshConfig>`
   - `#[serde(default, skip_serializing_if = "Option::is_none")] pub gossip_config: Option<GossipConfig>`
3. Added 5 unit tests: `manifest_without_mode_defaults_to_dag`, `manifest_with_mode_mesh_parses`, `manifest_with_mode_gossip_parses`, `manifest_mode_round_trip`, `manifest_mesh_config_omitted_when_none`.
4. Fixed one existing test (`manifest_session_scope_fields_omitted_when_empty_in_toml`) and one in `schema_roundtrip.rs` (`run_manifest_with_scoped_sessions_validates`) that constructed `RunManifest` literals — both needed the three new fields.
5. Ran `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` — snapshot updated and all 58 tests passed.

## Verification

- `cargo test -p assay-types --features orchestrate` — 58 passed, 0 failed
- `grep -c "mode\|mesh_config\|gossip_config" crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — returns 11 (multiple references to the three new properties in the JSON Schema)
- Backward-compat test `manifest_without_mode_defaults_to_dag`: TOML `[[sessions]]\nspec = "auth"` (no `mode`) parses to `mode: Dag, mesh_config: None, gossip_config: None` ✓
- Snapshot diff was additive-only: three new optional properties (`gossip_config`, `mesh_config`, `mode`) added to the schema with their `$ref` definitions

## Diagnostics

- Schema snapshot at `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` is the locked contract
- serde `deny_unknown_fields` on `RunManifest` remains active; misspelled field names produce immediate errors with the field name included
- Unit tests in `manifest.rs` cover all three parse paths and the omit-when-None serialization behavior

## Deviations

- Fixed `run_manifest_with_scoped_sessions_validates` in `schema_roundtrip.rs` — that test constructed a `RunManifest` literal and was missing the three new fields. Not in the task plan but necessary for compilation.

## Known Issues

none

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — added import, three new fields to `RunManifest`, 5 new unit tests, fixed one existing test literal
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — updated with three new optional properties
- `crates/assay-types/tests/schema_roundtrip.rs` — fixed one `RunManifest` literal to include new fields
