---
estimated_steps: 5
estimated_files: 5
---

# T01: Add RunManifest.state_backend field and write integration test contracts

**Slice:** S02 — LocalFsBackend implementation and orchestrator wiring
**Milestone:** M010

## Description

Establishes the backward-compat round-trip contract for `RunManifest.state_backend` and writes red-state integration tests that prove `LocalFsBackend` method bodies work end-to-end. This is the test-first contract — T02 makes the tests pass by implementing real method bodies.

## Steps

1. Add `state_backend: Option<StateBackendConfig>` to `RunManifest` in `crates/assay-types/src/manifest.rs` behind `#[cfg(feature = "orchestrate")]` with `#[serde(default, skip_serializing_if = "Option::is_none")]` — same pattern as `mesh_config` and `gossip_config`.
2. Run `INSTA_UPDATE=always cargo test -p assay-types --test schema_snapshots run_manifest_schema_snapshot` to regenerate the run-manifest schema snapshot. Review the diff to confirm `state_backend` appears as an optional field.
3. Add a backward-compat round-trip test in `crates/assay-core/tests/state_backend.rs`: deserialize a TOML manifest string without `state_backend`, assert it deserializes successfully and `state_backend` is `None`. Then serialize a manifest with `state_backend: Some(StateBackendConfig::LocalFs)`, round-trip it, assert equality.
4. Add integration tests for `LocalFsBackend` real method bodies in `crates/assay-core/tests/state_backend.rs`:
   - `test_local_fs_backend_push_and_read_state`: create a `LocalFsBackend`, call `push_session_event` with a constructed `OrchestratorStatus`, then `read_run_state` and assert the deserialized status matches.
   - `test_local_fs_backend_save_checkpoint_summary`: create a `LocalFsBackend`, call `save_checkpoint_summary` with a `TeamCheckpoint`, assert the checkpoint file exists at the expected path.
   - `test_local_fs_backend_send_and_poll_messages`: call `send_message`, then `poll_inbox`, assert the message is returned.
5. Verify the schema snapshot test passes and the new tests compile (they may fail because `LocalFsBackend` methods are still stubs — that's correct red state for T02).

## Must-Haves

- [ ] `RunManifest.state_backend` field exists with correct serde attributes and feature gate
- [ ] `run_manifest_schema_snapshot` updated and passing
- [ ] Backward-compat round-trip test: manifest without `state_backend` deserializes to `None`
- [ ] Integration tests for `push_session_event`/`read_run_state`, `save_checkpoint_summary`, `send_message`/`poll_inbox` are written
- [ ] `cargo test -p assay-types --test schema_snapshots` passes

## Verification

- `cargo test -p assay-types --test schema_snapshots run_manifest_schema_snapshot` — passes with updated snapshot
- `cargo test -p assay-core --features orchestrate --test state_backend -- backward_compat` — round-trip test passes (stubs return Ok)
- New integration tests compile (may not all pass yet — T02 makes them green)

## Observability Impact

- Signals added/changed: None (test infrastructure only)
- How a future agent inspects this: run `cargo test --test state_backend` to see which contracts pass/fail
- Failure state exposed: Test assertion messages name the exact field or method that failed

## Inputs

- `crates/assay-types/src/manifest.rs` — existing `RunManifest` struct with `mesh_config`/`gossip_config` pattern to follow
- `crates/assay-core/tests/state_backend.rs` — existing 6 contract tests from S01
- S01-SUMMARY forward intelligence: `run_manifest_schema_snapshot` was pre-existing-failing; update is safe

## Expected Output

- `crates/assay-types/src/manifest.rs` — `RunManifest` with new `state_backend` field
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — updated snapshot
- `crates/assay-core/tests/state_backend.rs` — 4+ new integration tests added to existing file
