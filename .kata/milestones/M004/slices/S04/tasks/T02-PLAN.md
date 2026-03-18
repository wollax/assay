---
estimated_steps: 5
estimated_files: 3
---

# T02: Add MCP Status Tests (Mesh/Gossip) and All-Modes Core Integration Test

**Slice:** S04 — Integration + Observability
**Milestone:** M004

## Description

Two coverage gaps remain after T01:

1. **MCP observability surface** — `orchestrate_status` already returns the full `OrchestratorStatus` including `mesh_status` and `gossip_status`, but there are no tests asserting these fields survive the round-trip through state.json. Two new MCP handler tests close this gap by writing realistic `mesh_status` / `gossip_status` payloads to state.json and asserting the response JSON contains them with correct values.

2. **All-modes regression suite** — mesh and gossip integration tests each live in their own file. There is no single place that runs DAG + Mesh + Gossip in one sweep. `integration_modes.rs` provides that regression target and proves S04's "all three modes have end-to-end coverage" milestone criterion.

Both are additive — no existing code changes, only new test code.

## Steps

1. **Add `orchestrate_status_returns_mesh_status` in `mcp_handlers.rs`**:
   - Follow the `orchestrate_status_reads_persisted_state_with_sessions` template (line 1618)
   - Build an `OrchestratorStatus` with `mesh_status: Some(MeshStatus { members: vec![MeshMemberStatus { name: "alpha".into(), state: MeshMemberState::Completed, last_heartbeat_at: None }], messages_routed: 3 })` and `gossip_status: None`
   - Serialize to state.json in a temp dir, call `server.orchestrate_status()`
   - Assert:
     - `response_json["status"]["mesh_status"]` is not null
     - `response_json["status"]["mesh_status"]["messages_routed"] == 3`
     - `response_json["status"]["mesh_status"]["members"][0]["name"] == "alpha"`
     - `response_json["status"]["mesh_status"]["members"][0]["state"] == "completed"`
     - `response_json["status"]["gossip_status"]` is null or absent (skip_serializing_if)
   - Required imports: `assay_types::orchestrate::{MeshMemberState, MeshMemberStatus, MeshStatus}`

2. **Add `orchestrate_status_returns_gossip_status` in `mcp_handlers.rs`**:
   - Build an `OrchestratorStatus` with `gossip_status: Some(GossipStatus { sessions_synthesized: 2, knowledge_manifest_path: std::path::PathBuf::from("/tmp/run/gossip/knowledge.json"), coordinator_rounds: 4 })` and `mesh_status: None`
   - Serialize to state.json, call `server.orchestrate_status()`
   - Assert:
     - `response_json["status"]["gossip_status"]["sessions_synthesized"] == 2`
     - `response_json["status"]["gossip_status"]["coordinator_rounds"] == 4`
     - `response_json["status"]["gossip_status"]["knowledge_manifest_path"]` is a non-empty string
     - `response_json["status"]["mesh_status"]` is null or absent

3. **Create `crates/assay-core/tests/integration_modes.rs`** with `#![cfg(feature = "orchestrate")]` and three tests:
   - **`test_all_modes_dag_executes_two_sessions`**: Use a real git repo (same `setup_git_repo()` pattern from orchestrate_integration.rs — `git init -b main` + initial commit + `.assay/orchestrator` dir). Create a 2-session manifest with `mode: OrchestratorMode::Dag` and `depends_on: vec![]` on both sessions. Call `run_orchestrated()` with a mock `success_result` runner. Assert `result.outcomes.len() == 2` and both are `SessionOutcome::Completed`. (DAG tests require a real git repo because the executor checks git state; copy the `setup_git_repo()` helper from orchestrate_integration.rs.)
   - **`test_all_modes_mesh_executes_two_sessions`**: 2-session `OrchestratorMode::Mesh` manifest, mock runner returns `success_result`. Assert `result.outcomes.len() == 2`, both `Completed`, `state.json` exists.
   - **`test_all_modes_gossip_executes_two_sessions`**: 2-session `OrchestratorMode::Gossip` manifest, mock runner returns `success_result`. Assert `result.outcomes.len() == 2`, both `Completed`, `knowledge.json` has 2 entries.
   - Define all helper functions locally in the file (cannot import from other test files in Rust). Copy `setup_git_repo()` from orchestrate_integration.rs for the DAG test; copy `setup_temp_dir()` + `make_pipeline_config()` from mesh/gossip integration tests for Mesh/Gossip. Add `fn make_dag_manifest(names: &[(&str, &str)]) -> RunManifest` for the DAG case. Each helper is ~10–20 lines.

4. **Check imports in `integration_modes.rs`**: needs `assay_core::orchestrate::executor::{OrchestratorConfig, run_orchestrated}`, `assay_core::orchestrate::mesh::run_mesh`, `assay_core::orchestrate::gossip::run_gossip`, `assay_types::{OrchestratorMode, RunManifest, ManifestSession}`, `assay_types::orchestrate::KnowledgeManifest`, and the pipeline types for `PipelineResult` / `PipelineError`.

5. **Run both test suites** and fix any compilation errors:
   - `cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status orchestrate_status_returns_gossip_status`
   - `cargo test -p assay-core --features orchestrate --test integration_modes`

## Must-Haves

- [ ] `orchestrate_status_returns_mesh_status` passes: `response_json["status"]["mesh_status"]["messages_routed"] == 3`
- [ ] `orchestrate_status_returns_gossip_status` passes: `response_json["status"]["gossip_status"]["sessions_synthesized"] == 2`
- [ ] `integration_modes.rs` compiles with `#![cfg(feature = "orchestrate")]` guard
- [ ] `test_all_modes_dag_executes_two_sessions` passes with 2 Completed outcomes
- [ ] `test_all_modes_mesh_executes_two_sessions` passes with 2 Completed outcomes and state.json written
- [ ] `test_all_modes_gossip_executes_two_sessions` passes with 2 Completed outcomes and knowledge.json having 2 entries

## Verification

- `cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status orchestrate_status_returns_gossip_status` — 2 new tests pass
- `cargo test -p assay-core --features orchestrate --test integration_modes` — 3 tests pass
- `cargo test -p assay-mcp` — all existing MCP tests still pass (no regressions)

## Observability Impact

- Signals added/changed: none at runtime — these are test-only additions
- How a future agent inspects this: `cargo test -p assay-mcp -- orchestrate_status_returns_mesh` — targeted run of mesh status surface test; `cargo test -p assay-core --features orchestrate --test integration_modes -- --nocapture` — verbose output shows which mode is being tested
- Failure state exposed: if `mesh_status` / `gossip_status` are accidentally removed from `OrchestratorStatus` or their serde attributes change, these tests fail immediately with clear assertion messages showing actual vs expected JSON field values

## Inputs

- `crates/assay-mcp/tests/mcp_handlers.rs:1618–1742` — `orchestrate_status_reads_persisted_state_with_sessions` is the exact template; copy structure, change `sessions` payload to `mesh_status: Some(MeshStatus{...})`
- `crates/assay-types/src/orchestrate.rs` — `MeshMemberState`, `MeshMemberStatus`, `MeshStatus`, `GossipStatus`, `KnowledgeManifest` field names and types
- `crates/assay-core/tests/mesh_integration.rs` — `setup_temp_dir()`, `make_pipeline_config()`, `make_mesh_manifest()`, `success_result()` helpers to copy into `integration_modes.rs`
- `crates/assay-core/tests/gossip_integration.rs` — `make_gossip_manifest()` helper to copy

## Expected Output

- `crates/assay-mcp/tests/mcp_handlers.rs` — 2 new `#[tokio::test] #[serial]` test functions added at end of orchestrate_status test block
- `crates/assay-core/tests/integration_modes.rs` — new file: `#![cfg(feature = "orchestrate")]`, local helpers, 3 test functions exercising DAG/Mesh/Gossip dispatch
