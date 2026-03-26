---
id: T01
parent: S02
milestone: M010
provides:
  - RunManifest.state_backend field with backward-compatible serde
  - Schema snapshot for orchestrate-gated RunManifest
  - Backward-compat round-trip tests for state_backend field
  - Red-state integration tests for LocalFsBackend push/read, checkpoint, messaging
key_files:
  - crates/assay-types/src/manifest.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap
  - crates/assay-core/tests/state_backend.rs
key_decisions:
  - Split schema snapshot into non-orchestrate (run-manifest-schema) and orchestrate (run-manifest-orchestrate-schema) variants to handle feature-gated fields without snapshot conflicts
patterns_established:
  - Feature-gated RunManifest fields need separate schema snapshot tests gated with cfg(feature) and cfg(not(feature))
observability_surfaces:
  - Run `cargo test --test state_backend` to see which LocalFsBackend contracts pass/fail
duration: ~10min
verification_result: passed
completed_at: 2026-03-26
blocker_discovered: false
---

# T01: Add RunManifest.state_backend field and write integration test contracts

**Added `state_backend: Option<StateBackendConfig>` to `RunManifest` with backward-compat serde, updated schema snapshots, and wrote 5 new tests (2 green backward-compat + 3 red-state integration contracts for T02).**

## What Happened

1. Added `state_backend: Option<StateBackendConfig>` to `RunManifest` behind `#[cfg(feature = "orchestrate")]` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, matching the `mesh_config`/`gossip_config` pattern.
2. Updated all struct literal constructions across the workspace (test files, orchestrate modules, CLI) to include `state_backend: None` — about 15 call sites.
3. Created a new `run_manifest_orchestrate_schema_snapshot` test (feature-gated) and gated the existing `run_manifest_schema_snapshot` with `#[cfg(not(feature = "orchestrate"))]` to avoid snapshot conflicts between feature flag states.
4. Added 2 backward-compat round-trip tests: manifest without `state_backend` deserializes to `None`; manifest with `Some(LocalFs)` survives TOML round-trip.
5. Added 3 red-state integration tests for LocalFsBackend: push+read state, save checkpoint, send+poll messages. These fail as expected because method bodies are stubs — T02 makes them green.

## Verification

- `cargo test -p assay-types --test schema_snapshots` — 47 passed (non-orchestrate)
- `cargo test -p assay-types --features orchestrate --test schema_snapshots` — 71 passed (orchestrate)
- `cargo test -p assay-core --features orchestrate --test state_backend -- backward_compat` — 2 passed (round-trip contracts)
- `cargo nextest run --workspace --no-fail-fast` — 1478 passed, 3 failed (exactly the 3 expected red-state tests)
- All slice-level verification tests pass: orchestrate_integration, mesh_integration, gossip_integration, orchestrate_spans, integration_modes

## Diagnostics

Run `cargo test -p assay-core --features orchestrate --test state_backend` to see contract status. The 3 integration tests (`push_and_read_state`, `save_checkpoint_summary`, `send_and_poll_messages`) will fail until T02 implements real method bodies.

## Deviations

- **Schema snapshot split**: The task plan said to update the existing `run_manifest_schema_snapshot`. Instead, I created a separate orchestrate-gated snapshot (`run-manifest-orchestrate-schema`) and gated the original with `#[cfg(not(feature = "orchestrate"))]`. This was necessary because `RunManifest`'s schema differs between feature flag states, and `cargo nextest run --workspace` runs without `--features orchestrate`.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — Added `state_backend` field and import
- `crates/assay-types/tests/schema_snapshots.rs` — Added orchestrate-gated snapshot test, gated base test
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — New snapshot with state_backend
- `crates/assay-core/tests/state_backend.rs` — Added 5 new tests (2 backward-compat + 3 integration)
- `crates/assay-core/tests/mesh_integration.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/tests/orchestrate_spans.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/tests/integration_modes.rs` — Added `state_backend: None` to struct literals
- `crates/assay-core/tests/pipeline_spans.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/tests/orchestrate_integration.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/tests/gossip_integration.rs` — Added `state_backend: None` to struct literal
- `crates/assay-core/src/orchestrate/mesh.rs` — Added `state_backend: None` to test helper
- `crates/assay-core/src/orchestrate/gossip.rs` — Added `state_backend: None` to test helper
- `crates/assay-cli/src/commands/run.rs` — Added `state_backend: None` to test helper
- `crates/assay-types/tests/schema_roundtrip.rs` — Added `state_backend: None` to struct literal
