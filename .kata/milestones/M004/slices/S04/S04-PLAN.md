# S04: Integration + Observability

**Goal:** All three modes have end-to-end integration coverage, `orchestrate_status` MCP tool surfaces mode-specific state (`mesh_status` / `gossip_status`), CLI shows mode in run output and populates session outcomes from real results, and `just ready` passes green with 0 warnings.
**Demo:** Running `execute_mesh()` or `execute_gossip()` with a 2-session manifest calls real session runners (not `unreachable!()`), prints "mode: mesh" / "mode: gossip" with per-session outcomes, returns a populated `OrchestrationResponse`, and writes `state.json` with mode-specific status. `orchestrate_status` MCP tests prove the status JSON contains `mesh_status` / `gossip_status`. An all-modes integration test in `integration_modes.rs` exercises DAG + Mesh + Gossip in one suite. `just ready` exits 0.

## Must-Haves

- `execute_mesh()` in `run.rs` uses a real HarnessWriter session runner (not `unreachable!()`), prints "mode: mesh" and per-session outcomes to stderr, populates `OrchestrationResponse.sessions` and `summary` from `orch_result.outcomes`
- `execute_gossip()` in `run.rs` uses a real HarnessWriter session runner (not `unreachable!()`), prints "mode: gossip" and per-session outcomes to stderr, populates `OrchestrationResponse.sessions` and `summary` from `orch_result.outcomes`
- Two new MCP tests in `mcp_handlers.rs`: `orchestrate_status_returns_mesh_status` and `orchestrate_status_returns_gossip_status` — write `mesh_status: Some(MeshStatus{...})` / `gossip_status: Some(GossipStatus{...})` into `state.json`, call `orchestrate_status`, assert `response_json["status"]["mesh_status"]` / `["gossip_status"]` is non-null with correct field values
- New `crates/assay-core/tests/integration_modes.rs` exercises all three modes in one file: DAG mode (reuses helpers from orchestrate_integration), Mesh mode (reuses `make_mesh_manifest` from mesh_integration helpers), Gossip mode (reuses `make_gossip_manifest` from gossip_integration helpers)
- `just ready` passes: fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓ with all existing tests still passing

## Proof Level

- This slice proves: integration — CLI session runner wiring (contract), MCP observability surface (contract), all-modes core coverage (integration)
- Real runtime required: no (mock session runners in all integration tests)
- Human/UAT required: no (real Claude coordination is manual UAT, deferred)

## Verification

- `cargo test -p assay-cli` — all existing CLI tests pass; no panics from unreachable!()
- `cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status orchestrate_status_returns_gossip_status` — two new tests pass; response JSON contains `mesh_status` / `gossip_status` non-null
- `cargo test -p assay-core --features orchestrate --test integration_modes` — new all-modes test file compiles and all tests pass (dag, mesh, gossip modes each exercised)
- `just ready` — exits 0 with 0 warnings; test count ≥ 1270 (1264 existing + at least 6 new)

## Observability / Diagnostics

- Runtime signals: CLI stderr now prints `"mode: mesh"` / `"mode: gossip"` on entry; per-session `[✓]`/`[✗]`/`[−]` outcome lines (same as DAG path)
- Inspection surfaces:
  - `assay run manifest.toml 2>&1 | grep "mode:"` — confirms mode printed at startup
  - `assay run manifest.toml --json | jq '.sessions | length'` — confirms outcomes populated (not empty `[]`)
  - `orchestrate_status` MCP response `["status"]["mesh_status"]` / `["status"]["gossip_status"]` — readable after any mesh/gossip run
- Failure visibility: if execute_mesh/execute_gossip panics (unreachable!() still present), the process aborts with a clear backtrace; replaced with real runner, failures surface as `SessionOutcome::Failed` with `error.message` in the response JSON
- Redaction constraints: none — no secrets or PII in session outcome messages

## Integration Closure

- Upstream surfaces consumed:
  - `assay_core::orchestrate::mesh::run_mesh()` — full implementation from S02
  - `assay_core::orchestrate::gossip::run_gossip()` — full implementation from S03
  - `assay_types::orchestrate::{MeshStatus, GossipStatus}` — from S02/S03
  - `execute_orchestrated()` in `run.rs` — template for HarnessWriter session runner construction
- New wiring introduced in this slice:
  - `execute_mesh()` and `execute_gossip()` now compose real `HarnessWriter` closures and pass them to `run_mesh()`/`run_gossip()` — closing the CLI→core execution loop for all three modes
  - MCP test coverage for `mesh_status`/`gossip_status` in `orchestrate_status` response
  - All-modes integration test suite in `integration_modes.rs`
- What remains before the milestone is truly usable end-to-end:
  - MCP `orchestrate_run` mesh/gossip stubs in `server.rs` still use `unreachable!()` — left for post-M004 since MCP integration tests use `orchestrate_status` directly against written `state.json` (real git worktrees unavailable in MCP test environment)
  - Real Claude agent coordination (manual UAT only)
  - Heartbeat polling for `Suspect` state transitions (deferred per S02 known limitations)
  - OTel instrumentation (deferred to M005+ per R027)

## Tasks

- [x] **T01: Rewrite `execute_mesh()` and `execute_gossip()` CLI stubs with real session runners** `est:30m`
  - Why: Current stubs use `unreachable!()` session runners and return hardcoded empty sessions/summary — calling them with any manifest would panic. This is the primary S04 implementation work.
  - Files: `crates/assay-cli/src/commands/run.rs`
  - Do:
    1. Copy the HarnessWriter closure from `execute_orchestrated()` (lines ~385–400) into both `execute_mesh()` and `execute_gossip()` — replace the `unreachable!()` closure.
    2. In `execute_mesh()`: (a) change the leading eprintln to `"mode: mesh — {} session(s)"`, (b) after `run_mesh()` returns, iterate `orch_result.outcomes` the same way `execute_orchestrated` does to build `session_results`, `completed_count`, `failed_count`, `skipped_count`, (c) populate `OrchestrationResponse.sessions` and `summary` from these counts.
    3. In `execute_gossip()`: same pattern as step 2 but with `"mode: gossip — {} session(s)"`.
    4. Keep `merge_report` as an empty `MergeReport` (no merge phase for mesh/gossip per research doc recommendation — least change, D005 additive, no new types needed).
    5. Add or update the existing `mode_mesh_bypasses_needs_orchestration` and `mode_gossip_bypasses_needs_orchestration` CLI unit tests to assert mode is correctly set on the manifest (these already exist and pass — verify they still pass).
    6. Add two new CLI unit tests: `execute_mesh_response_shape` and `execute_gossip_response_shape` — create a `RunManifest` with mode=Mesh/Gossip and assert the `OrchestrationResponse` struct can be serialized to JSON (doesn't need real runner execution; tests the struct shape).
  - Verify: `cargo test -p assay-cli` passes with no panics; `cargo clippy -p assay-cli --all-targets --features orchestrate -- -D warnings` exits 0
  - Done when: `execute_mesh()` and `execute_gossip()` compile without `unreachable!()` session runners, print mode to stderr, and return a populated `OrchestrationResponse`; all CLI tests pass; 0 clippy warnings

- [x] **T02: Add MCP status tests (mesh/gossip) and all-modes core integration test** `est:45m`
  - Why: Proves that `orchestrate_status` correctly surfaces `mesh_status` / `gossip_status` to MCP callers, and provides a single place that exercises all three execution modes together for long-term regression value.
  - Files: `crates/assay-mcp/tests/mcp_handlers.rs`, `crates/assay-core/tests/integration_modes.rs`
  - Do:
    1. In `mcp_handlers.rs`, add `orchestrate_status_returns_mesh_status`:
       - Write `state.json` with `mesh_status: Some(MeshStatus { members: vec![MeshMemberStatus { name: "alpha".into(), state: MeshMemberState::Completed, last_heartbeat_at: None }], messages_routed: 3 })` and `gossip_status: None`
       - Call `orchestrate_status`, assert `response_json["status"]["mesh_status"]` is non-null
       - Assert `response_json["status"]["mesh_status"]["messages_routed"] == 3`
       - Assert `response_json["status"]["gossip_status"]` is null/absent
    2. In `mcp_handlers.rs`, add `orchestrate_status_returns_gossip_status`:
       - Write `state.json` with `gossip_status: Some(GossipStatus { sessions_synthesized: 2, knowledge_manifest_path: PathBuf::from("/tmp/knowledge.json"), coordinator_rounds: 4 })` and `mesh_status: None`
       - Call `orchestrate_status`, assert `response_json["status"]["gossip_status"]["sessions_synthesized"] == 2`
       - Assert `response_json["status"]["gossip_status"]["coordinator_rounds"] == 4`
       - Assert `response_json["status"]["mesh_status"]` is null/absent
    3. Create `crates/assay-core/tests/integration_modes.rs` with three tests:
       - `test_all_modes_dag_executes_two_sessions`: init a git repo (copy `setup_git_repo()` from orchestrate_integration), run a 2-session DAG manifest with no `depends_on`, assert both sessions complete (reuse `make_pipeline_config` / `success_result` style helpers).
       - `test_all_modes_mesh_runs_parallel`: use `make_mesh_manifest` helper pattern (no git needed), 2 mock sessions, assert `run_mesh()` returns 2 outcomes both Completed.
       - `test_all_modes_gossip_populates_manifest`: use `make_gossip_manifest` helper pattern (no git needed), 2 mock sessions, assert `run_gossip()` returns and `knowledge.json` exists with 2 entries.
    4. Ensure `integration_modes.rs` is guarded with `#![cfg(feature = "orchestrate")]` and imports the shared helpers by redefining them locally (they can't be imported from other test files — copy the minimal setup functions).
  - Verify: `cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status orchestrate_status_returns_gossip_status` passes; `cargo test -p assay-core --features orchestrate --test integration_modes` passes with all 3 tests green
  - Done when: 2 new MCP tests + 3 new core integration tests all pass; `response_json["status"]["mesh_status"]["messages_routed"]` == 3 asserted; `response_json["status"]["gossip_status"]["sessions_synthesized"]` == 2 asserted; all-modes file compiles and runs cleanly

- [x] **T03: `just ready` final pass and write S04-SUMMARY.md** `est:15m`
  - Why: Confirms the full workspace compiles with 0 warnings, all tests pass, deny checks pass, and the slice is officially closed out.
  - Files: `crates/assay-cli/src/commands/run.rs` (fix any warnings), `.kata/milestones/M004/slices/S04/S04-SUMMARY.md`, `.kata/STATE.md`, `.kata/DECISIONS.md`
  - Do:
    1. Run `just ready` and fix any `cargo fmt` issues, clippy warnings, or test failures.
    2. Verify test count ≥ 1270 (1264 + at least 6 new tests from T01–T02).
    3. Write `S04-SUMMARY.md` following the standard slice summary format (id, provides, requires, affects, key_files, key_decisions, patterns_established, observability_surfaces, verification_result, etc.).
    4. Append D061 to `.kata/DECISIONS.md` (see Integration Closure note — execute_mesh/execute_gossip use HarnessWriter pattern without merge phase).
    5. Update `.kata/STATE.md`: mark S04 complete, update M004 status, update test count.
    6. Commit: `feat(S04): wire execute_mesh/execute_gossip real session runners, add all-modes integration and MCP status tests`.
  - Verify: `just ready` exits 0 with 0 warnings; `grep -c "test_" .kata/milestones/M004/slices/S04/S04-SUMMARY.md` shows tests listed; `git log --oneline -1` shows the feat commit
  - Done when: `just ready` exits 0, S04-SUMMARY.md committed, STATE.md shows M004 complete

## Files Likely Touched

- `crates/assay-cli/src/commands/run.rs` — rewrite execute_mesh/execute_gossip, add 2 unit tests
- `crates/assay-mcp/tests/mcp_handlers.rs` — add 2 orchestrate_status mesh/gossip tests
- `crates/assay-core/tests/integration_modes.rs` — new file: all-modes integration test suite
- `.kata/milestones/M004/slices/S04/S04-SUMMARY.md` — new file
- `.kata/STATE.md` — mark S04 and M004 complete
- `.kata/DECISIONS.md` — append D061
