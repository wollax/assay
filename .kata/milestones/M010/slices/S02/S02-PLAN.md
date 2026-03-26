# S02: LocalFsBackend implementation and orchestrator wiring

**Goal:** All orchestrator, mesh, gossip, and checkpoint writes flow through `LocalFsBackend` via `Arc<dyn StateBackend>` on `OrchestratorConfig`; `RunManifest.state_backend` field added with backward-compatible serde; existing integration tests pass unchanged.
**Demo:** `just ready` green with all ~1476 tests passing; round-trip test proves `RunManifest` without `state_backend` deserializes identically; every `persist_state()` callsite replaced by `backend.push_session_event()`.

## Must-Haves

- `RunManifest.state_backend: Option<StateBackendConfig>` with `serde(default, skip_serializing_if = "Option::is_none")`, feature-gated behind `orchestrate`; backward-compat round-trip test passes
- `OrchestratorConfig.backend: Arc<dyn StateBackend>` field; `OrchestratorConfig` no longer derives `Clone` (replaced with manual `Clone` or uses `Arc` which is `Clone`)
- `LocalFsBackend::push_session_event` delegates to the atomic tempfile-rename pattern from `persist_state()`
- `LocalFsBackend::read_run_state` deserializes `state.json` from `run_dir`
- `LocalFsBackend::save_checkpoint_summary` delegates to existing `save_checkpoint()` in `checkpoint::persistence`
- All 15 `persist_state()` callsites in executor.rs, mesh.rs, gossip.rs replaced with `config.backend.push_session_event()`
- `persist_state()` removed or made private to `LocalFsBackend` (no longer `pub(crate)`)
- All existing integration tests (`orchestrate_integration.rs`, `mesh_integration.rs`, `gossip_integration.rs`, `orchestrate_spans.rs`, `integration_modes.rs`) pass unchanged
- All CLI and MCP `OrchestratorConfig` construction sites updated with `backend` field
- `run_manifest_schema_snapshot` updated with the new `state_backend` field
- `just ready` green (fmt + lint + test + deny)

## Proof Level

- This slice proves: integration
- Real runtime required: no (mock session runners used in integration tests, same as today)
- Human/UAT required: no

## Verification

- `cargo test -p assay-types --test schema_snapshots run_manifest_schema_snapshot` — snapshot includes `state_backend` field
- `cargo test -p assay-core --features orchestrate --test state_backend` — contract tests still pass + new backward-compat round-trip test
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — all existing tests pass
- `cargo test -p assay-core --features orchestrate --test mesh_integration` — all existing tests pass
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — all existing tests pass
- `cargo test -p assay-core --features orchestrate --test orchestrate_spans` — all existing tests pass
- `cargo test -p assay-core --features orchestrate --test integration_modes` — all existing tests pass
- `cargo test --workspace` — all ~1476 tests pass
- `just ready` — green

## Observability / Diagnostics

- Runtime signals: `LocalFsBackend` stub `tracing::warn!` messages replaced with real write operations; errors propagated as `AssayError` with path/operation context
- Inspection surfaces: `state.json` written by backend (same path as before); `backend.read_run_state()` reads it back
- Failure visibility: `AssayError::io` and `AssayError::json` carry file path + operation label on every persistence failure
- Redaction constraints: none (no secrets in orchestrator state)

## Integration Closure

- Upstream surfaces consumed: `StateBackend` trait, `CapabilitySet`, `LocalFsBackend` skeleton, `StateBackendConfig` from S01
- New wiring introduced in this slice: `Arc<dyn StateBackend>` on `OrchestratorConfig`; all executor/mesh/gossip status writes flow through the backend; `RunManifest.state_backend` field selects backend at manifest load time
- What remains before the milestone is truly usable end-to-end: S03 (CapabilitySet degradation checks), S04 (smelt-agent plugin documentation)

## Tasks

- [x] **T01: Add RunManifest.state_backend field and write integration test contracts** `est:30m`
  - Why: Establishes the backward-compat round-trip contract and the `OrchestratorConfig` shape with `Arc<dyn StateBackend>` — the test-first contract that all subsequent tasks must satisfy
  - Files: `crates/assay-types/src/manifest.rs`, `crates/assay-core/tests/state_backend.rs`, `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap`
  - Do: Add `state_backend: Option<StateBackendConfig>` to `RunManifest` behind `orchestrate` feature with `serde(default, skip_serializing_if)`. Add backward-compat round-trip test (manifest without field deserializes). Update `run_manifest_schema_snapshot`. Add integration test proving `LocalFsBackend::push_session_event` writes a readable `state.json`, `read_run_state` reads it back, and `save_checkpoint_summary` delegates to existing persistence.
  - Verify: `cargo test -p assay-types --test schema_snapshots run_manifest_schema_snapshot` passes; new contract tests compile but some may fail (red state for T02)
  - Done when: `RunManifest.state_backend` field exists, schema snapshot updated, round-trip test passes, integration tests written

- [x] **T02: Implement LocalFsBackend real method bodies** `est:30m`
  - Why: Replaces stub implementations with real filesystem persistence — the core functionality that all callsite wiring depends on
  - Files: `crates/assay-core/src/state_backend.rs`
  - Do: Implement `push_session_event` using atomic tempfile-rename (same pattern as `persist_state`). Implement `read_run_state` using `serde_json::from_str`. Implement `save_checkpoint_summary` delegating to `checkpoint::persistence::save_checkpoint`. Implement `send_message` and `poll_inbox` with filesystem ops (write file to path, read+delete from dir). Implement `annotate_run` writing a manifest path to a file. Remove stub `tracing::warn!` messages.
  - Verify: `cargo test -p assay-core --features orchestrate --test state_backend` — all contract tests pass (green)
  - Done when: All 7 `LocalFsBackend` methods have real implementations; contract tests pass

- [x] **T03: Wire Arc<dyn StateBackend> into OrchestratorConfig and replace all persist_state callsites** `est:45m`
  - Why: The central wiring task — connects the backend to all three executors, replacing 15 direct `persist_state()` calls with `backend.push_session_event()`
  - Files: `crates/assay-core/src/orchestrate/executor.rs`, `crates/assay-core/src/orchestrate/mesh.rs`, `crates/assay-core/src/orchestrate/gossip.rs`
  - Do: Add `backend: Arc<dyn StateBackend>` to `OrchestratorConfig`. Remove `#[derive(Clone)]`, add manual `Clone` impl using `Arc::clone`. Replace `Default` impl to use a `LocalFsBackend::new` with a default temp path (or remove `Default` and fix all callsites). Replace all 15 `persist_state()` callsites with `config.backend.push_session_event()`. Make `persist_state` function private or remove it. Ensure `Arc` is cloned into `thread::scope` workers (gossip coordinator thread, mesh routing thread).
  - Verify: `cargo test -p assay-core --features orchestrate --test orchestrate_integration` + `mesh_integration` + `gossip_integration` + `orchestrate_spans` + `integration_modes` all pass
  - Done when: Zero `persist_state()` calls remain in executor/mesh/gossip; all integration tests pass

- [ ] **T04: Update CLI, MCP, and TUI OrchestratorConfig construction sites and run just ready** `est:30m`
  - Why: Completes the wiring by updating all external construction sites (CLI, MCP, TUI, embedded tests) and proves the full workspace compiles and passes
  - Files: `crates/assay-cli/src/commands/run.rs`, `crates/assay-mcp/src/server.rs`, `crates/assay-tui/src/app.rs` (if any)
  - Do: Update 3 `OrchestratorConfig` construction sites in CLI, 3 in MCP server to pass `backend: Arc::new(LocalFsBackend::new(assay_dir.clone()))`. Update any remaining test files that construct `OrchestratorConfig` with struct literals or `::default()`. Run `INSTA_UPDATE=always cargo test` to accept any snapshot changes. Run `just ready` for final green.
  - Verify: `just ready` green; `cargo test --workspace` — all tests pass
  - Done when: `just ready` green; zero compile errors; zero `persist_state` references outside `state_backend.rs`

## Files Likely Touched

- `crates/assay-types/src/manifest.rs` — add `state_backend` field
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — updated snapshot
- `crates/assay-core/src/state_backend.rs` — real method implementations
- `crates/assay-core/src/orchestrate/executor.rs` — `OrchestratorConfig` + `Arc` backend + callsite replacement
- `crates/assay-core/src/orchestrate/mesh.rs` — callsite replacement
- `crates/assay-core/src/orchestrate/gossip.rs` — callsite replacement
- `crates/assay-core/tests/state_backend.rs` — new integration tests
- `crates/assay-core/tests/orchestrate_integration.rs` — `OrchestratorConfig` construction update
- `crates/assay-core/tests/mesh_integration.rs` — `OrchestratorConfig` construction update
- `crates/assay-core/tests/gossip_integration.rs` — `OrchestratorConfig` construction update
- `crates/assay-core/tests/orchestrate_spans.rs` — `OrchestratorConfig` construction update
- `crates/assay-core/tests/integration_modes.rs` — `OrchestratorConfig` construction update
- `crates/assay-cli/src/commands/run.rs` — `OrchestratorConfig` construction update
- `crates/assay-mcp/src/server.rs` — `OrchestratorConfig` construction update
