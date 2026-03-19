# S04: Integration + Observability — UAT

**Milestone:** M004
**Written:** 2026-03-18

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All three execution modes (DAG, Mesh, Gossip) are exercised by automated integration tests using mock session runners. The mock runner pattern is identical to S02/S03 (already accepted as the verification boundary). Real Claude agent coordination is a separate manual UAT concern deferred to post-M004. MCP observability is verified via state.json round-trip tests — the same pattern used for all other MCP surface tests in assay-mcp.

## Preconditions

- `just ready` exits 0 (confirmed: 1271 tests, 0 warnings)
- `crates/assay-core/tests/integration_modes.rs` exists and all 3 tests pass
- `crates/assay-mcp/tests/mcp_handlers.rs` contains `orchestrate_status_returns_mesh_status` and `orchestrate_status_returns_gossip_status`
- `execute_mesh()` and `execute_gossip()` in `crates/assay-cli/src/commands/run.rs` contain no `unreachable!()` calls

## Smoke Test

```
cargo test -p assay-core --features orchestrate --test integration_modes -- --nocapture
```

Expected: 3 tests pass (dag, mesh, gossip). Output shows each mode dispatcher invoked, outcomes counted.

## Test Cases

### 1. execute_mesh no longer panics on real manifest

1. Build a `RunManifest` with `mode = OrchestratorMode::Mesh` and two sessions.
2. Call `execute_mesh()` with the manifest.
3. **Expected:** Function returns without panic; `OrchestrationResponse.sessions` has 2 entries; stderr includes `"mode: mesh — 2 session(s)"`.

Verified by: `cargo test -p assay-cli -- execute_mesh_output_mode_label` (guard test).

### 2. execute_gossip no longer panics on real manifest

1. Build a `RunManifest` with `mode = OrchestratorMode::Gossip` and two sessions.
2. Call `execute_gossip()` with the manifest.
3. **Expected:** Function returns without panic; `OrchestrationResponse.sessions` has 2 entries; stderr includes `"mode: gossip — 2 session(s)"`.

Verified by: `cargo test -p assay-cli -- execute_gossip_output_mode_label` (guard test).

### 3. orchestrate_status surfaces mesh_status

1. Write `state.json` with `mesh_status: Some(MeshStatus { members: [...], messages_routed: 3 })` and `gossip_status: None`.
2. Call `server.orchestrate_status()`.
3. **Expected:** Response JSON `["status"]["mesh_status"]["messages_routed"] == 3`; `["status"]["mesh_status"]["members"][0]["state"] == "completed"`; `["status"]["gossip_status"]` is null.

Verified by: `cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status`.

### 4. orchestrate_status surfaces gossip_status

1. Write `state.json` with `gossip_status: Some(GossipStatus { sessions_synthesized: 2, coordinator_rounds: 4, ... })` and `mesh_status: None`.
2. Call `server.orchestrate_status()`.
3. **Expected:** Response JSON `["status"]["gossip_status"]["sessions_synthesized"] == 2`; `["status"]["gossip_status"]["coordinator_rounds"] == 4`; `["status"]["mesh_status"]` is null.

Verified by: `cargo test -p assay-mcp -- orchestrate_status_returns_gossip_status`.

### 5. All-modes integration regression

1. Run `cargo test -p assay-core --features orchestrate --test integration_modes`.
2. **Expected:** All 3 tests pass:
   - `test_all_modes_dag_executes_two_sessions` — 2 Completed outcomes
   - `test_all_modes_mesh_executes_two_sessions` — 2 Completed outcomes
   - `test_all_modes_gossip_executes_two_sessions` — 2 Completed outcomes, `knowledge.json` has 2 entries

## Edge Cases

### Gossip knowledge.json has correct entry count

1. In `test_all_modes_gossip_executes_two_sessions`, the test deserializes `knowledge.json` from the temp dir.
2. **Expected:** `knowledge_manifest.entries.len() == 2` (one per completed mock session).

### Mesh state.json captures member states

1. In `test_all_modes_mesh_executes_two_sessions`, the test reads `state.json` from the run dir.
2. **Expected:** `mesh_status.members` contains 2 entries with state `Completed`.

### Missing mode-specific status is null in response

1. A DAG run writes `state.json` with `mesh_status: None` and `gossip_status: None`.
2. `orchestrate_status` is called.
3. **Expected:** Both `["status"]["mesh_status"]` and `["status"]["gossip_status"]` are null in the response (not missing keys, not empty objects).

## Failure Signals

- `unreachable!()` panic in `execute_mesh`/`execute_gossip` → indicates stubs were not replaced; check `run.rs`
- `cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status` fails → indicates `OrchestratorStatus.mesh_status` serde changed or `MeshStatus` fields renamed
- `test_all_modes_gossip_executes_two_sessions` fails with "knowledge.json not found" → `run_gossip()` coordinator thread not writing the manifest; check gossip.rs
- `just ready` fails with warnings → new code in run.rs or mcp_handlers.rs added dead code or unused imports

## Requirements Proved By This UAT

- R034 (OrchestratorMode selection) — all three modes dispatch correctly through CLI execute_* functions with real runners
- R035 (Mesh mode execution) — `run_mesh()` called with real HarnessWriter closure, sessions complete with Completed outcomes
- R036 (Mesh peer messaging) — `messages_routed` surfaces in `orchestrate_status` MCP response via `mesh_status` field
- R037 (Gossip mode execution) — `run_gossip()` called with real HarnessWriter closure, coordinator writes `knowledge.json`
- R038 (Gossip knowledge manifest injection) — `knowledge.json` populated with correct entry count verified in integration test

## Not Proven By This UAT

- Real Claude agents actually reading and acting on roster/knowledge manifest paths — requires live runtime with real `claude -p` invocations (manual UAT only, deferred post-M004)
- Heartbeat-based `Suspect` membership state transitions — deferred (heartbeat polling not implemented in S02/S03)
- MCP `orchestrate_run` mesh/gossip paths — `server.rs` still uses `unreachable!()` for those entry points; MCP integration test environment cannot provision real git worktrees
- OTel instrumentation (R027) — deferred to M005+
- `assay run manifest.toml 2>&1 | grep "mode:"` live CLI output — not run in automated tests; verifiable manually once a real manifest and agent are available

## Notes for Tester

- All automated tests use mock session runners that return immediate `SessionResult::success()` — no real agent invocations happen in CI.
- The `#[serial]` attributes added to two `server.rs` unit tests (`context_diagnose_no_session_dir_returns_error`, `estimate_tokens_no_session_dir_returns_error`) are correctness fixes for pre-existing race conditions exposed by the parallel test suite; they do not change production behavior.
- If a future test uses `set_current_dir` in a unit test without `#[serial]`, it will silently race with these tests. The pattern is now documented in S04-SUMMARY.md forward intelligence.
