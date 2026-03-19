# S04: Integration + Observability — Research

**Date:** 2026-03-18

## Summary

S04 is the lightest slice in M004. All the heavy lifting is done: `run_mesh()` and `run_gossip()` are fully implemented, `mesh_status` and `gossip_status` are persisted to `state.json`, and all schema snapshots are locked. S04's job is to wire these surfaces into the CLI and MCP tool so callers can actually observe mode-specific state, and to add an end-to-end integration test covering all three modes in one suite.

**Three concrete deliverables:**
1. **CLI `execute_mesh` / `execute_gossip` rewrites** — the current stubs in `assay-cli/src/commands/run.rs` do not read `OrchestratorResult` session outcomes or print mode in output. They should print "mode: mesh" / "mode: gossip" and surface per-session outcomes and mesh/gossip status. The `OrchestrationResponse` they return currently has hardcoded empty sessions and summary — these should be populated from `orch_result.outcomes`.
2. **`orchestrate_status` MCP response already works** — `OrchestratorStatus` already has `mesh_status` and `gossip_status` fields; the `orchestrate_status` handler deserializes from `state.json` and returns the full struct. No code change needed here — just a test proving mesh_status / gossip_status are surfaced in the response JSON.
3. **End-to-end integration test** — a new test file covering DAG + Mesh + Gossip mode dispatch in one place, exercising the full stack from `RunManifest` through to `state.json` assertions.

Current test count: **1264 passing**, `just ready` green.

## Recommendation

**Execute in 3 tasks:**
- **T01** — Rewrite `execute_mesh()` and `execute_gossip()` in `run.rs` to call real session runners (like `execute_orchestrated` does), print mode in output, and populate response from outcomes.
- **T02** — Write integration tests: (a) `orchestrate_status` returns `mesh_status` / `gossip_status` in MCP test file; (b) new `crates/assay-core/tests/integration_modes.rs` with all-modes coverage.
- **T03** — `just ready` pass, update slice summary.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Session runner closure for CLI | `execute_orchestrated()` in `run.rs` lines ~360–400 | Exact pattern for constructing HarnessWriter + calling run_session; copy-paste with mesh/gossip path |
| Printing mode-specific output | Pattern already established in `execute_orchestrated()` | Same eprintln!/json format used throughout |
| MCP `orchestrate_status` mesh/gossip test | `orchestrate_status_reads_persisted_state_with_sessions` in `mcp_handlers.rs` | Existing test writes state.json, calls tool, asserts on JSON — copy with `mesh_status: Some(...)` |
| `make_mesh_manifest` / `make_gossip_manifest` helpers | `mesh_integration.rs` and `gossip_integration.rs` | Reuse directly in the all-modes integration test |

## Existing Code and Patterns

- `crates/assay-cli/src/commands/run.rs:587–740` — `execute_mesh()` and `execute_gossip()` stubs. Both call `run_mesh()` / `run_gossip()` with `unreachable!()` session runners and return hardcoded empty results. Must be replaced with real session runners that call `run_session` (same pattern as `execute_orchestrated`).
- `crates/assay-cli/src/commands/run.rs:170–220` — `execute_orchestrated()` — the template. Constructs session runner closure capturing HarnessWriter, calls `run_orchestrated`, formats `OrchestrationResponse`. Mesh/Gossip don't need the merge phase (phases 2 & 3), but should use the same runner construction.
- `crates/assay-mcp/src/server.rs:3171–3250` — `orchestrate_status` handler. Already returns `OrchestratorStatus` in full (including `mesh_status` / `gossip_status`). No code change needed.
- `crates/assay-mcp/src/server.rs:2900–2970` — mesh/gossip stubs in `orchestrate_run`. These also use `unreachable!()` session runners. Parallel fix to CLI stubs — but note: the MCP path is harder to test with real git (no worktree available), so fixing these is **optional for S04** given MCP integration tests use `orchestrate_status` directly against a written `state.json`.
- `crates/assay-mcp/tests/mcp_handlers.rs:1618–1742` — `orchestrate_status_reads_persisted_state_with_sessions` — template for new mesh/gossip MCP status tests. Write `mesh_status: Some(MeshStatus { ... })` into state.json, call `orchestrate_status`, assert `response_json["status"]["mesh_status"]` is non-null.
- `crates/assay-core/tests/mesh_integration.rs` — `make_mesh_manifest`, `setup_temp_dir`, `make_pipeline_config`, `success_result` helpers — reuse in all-modes integration test.
- `crates/assay-core/tests/gossip_integration.rs` — `make_gossip_manifest` — reuse in all-modes integration test.
- `crates/assay-types/src/orchestrate.rs` — `MeshStatus { members, messages_routed }`, `GossipStatus { sessions_synthesized, knowledge_manifest_path, coordinator_rounds }` — field names to use in test assertions.

## Constraints

- **D005 additive-only** — `orchestrate_status` already returns the full `OrchestratorStatus`; no field removal or signature change needed.
- **D032 SessionRunner as closure** — mesh/gossip CLI stubs must use the same `|session, pipe_cfg| -> Result<PipelineResult, PipelineError>` pattern as `execute_orchestrated`, not hardcode `unreachable!()`.
- **D052 free functions** — no executor trait. CLI stubs call `run_mesh()` / `run_gossip()` directly.
- **Sync threading (D017)** — CLI `execute_mesh` / `execute_gossip` are sync (no `spawn_blocking` needed — they're called from sync `execute()` already). MCP stubs in `orchestrate_run` already use `spawn_blocking` correctly.
- **No merge phase for Mesh/Gossip** — Neither mode runs the 3-phase (orchestrate + checkout + merge) flow. The CLI and MCP stubs should simply return `run_id`, session outcomes, and mode-specific status — no merge report.
- **`OrchestrationResponse` has mandatory `merge_report`** — the current CLI response type uses `merge_report: assay_types::MergeReport` (non-optional). For Mesh/Gossip the empty stub creates a zero-filled `MergeReport`. Consider whether to keep this shape or create separate response types. **Recommendation:** reuse `OrchestrationResponse` with an empty `MergeReport` for now (least change, no new types needed, D005 additive).

## Common Pitfalls

- **`unreachable!()` session runner in CLI stubs** — the current stubs call `run_mesh()`/`run_gossip()` with a `|| unreachable!()` runner because the real executors were stubs in S01 that never called runners. Now that `run_mesh()` and `run_gossip()` are full implementations, passing `unreachable!()` would panic on the first session. The CLI must construct a real session runner (copy from `execute_orchestrated`).
- **MCP stubs also use `unreachable!()`** — same problem in `server.rs` mesh/gossip branches. However, since MCP integration tests can't easily exercise real git+worktrees, the MCP path may be left with the stub for now. The new MCP test should use the `orchestrate_status` route (write state.json manually) rather than calling `orchestrate_run` end-to-end.
- **MeshStatus construction in MCP test** — `MeshStatus { members: vec![], messages_routed: 0 }` is valid for a status assertion test; no need for real sessions to populate it.
- **`gossip_status.knowledge_manifest_path` is a `PathBuf`** — will serialize as a JSON string; test assertion should use `.as_str()` not `.as_object()`.
- **All 9+ `OrchestratorStatus` construction sites** — S02 and S03 already patched all sites with `mesh_status: None, gossip_status: None`. S04 should not need to touch these.

## Open Risks

- The CLI `execute_mesh` / `execute_gossip` rewrites require the full session runner setup (HarnessWriter, etc.). If this introduces a compilation error or warning, it will block `just ready`. Low risk since `execute_orchestrated` is the exact template.
- `just ready` currently passes with 1264 tests. S04 adds integration tests in `assay-core/tests/` which are only compiled with `--features orchestrate` — they don't count toward the plain `cargo test` run. Verify `just ready` runs with `--features orchestrate` or that test counts are reconciled.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust (std) | — | None needed — all patterns already established in codebase |

## Sources

- S02-SUMMARY.md — Forward Intelligence section: "S04 must wire `mesh_status` from state.json into the `orchestrate_status` MCP tool response"
- S03-SUMMARY.md — Forward Intelligence section: "S04 must wire `gossip_status` into the `orchestrate_status` MCP tool's response surface and surface mode in CLI run output"
- `crates/assay-cli/src/commands/run.rs` — current mesh/gossip stubs and `execute_orchestrated` template
- `crates/assay-mcp/src/server.rs` — `orchestrate_status` handler already correct, mesh/gossip stubs in `orchestrate_run`
- `crates/assay-mcp/tests/mcp_handlers.rs` — `orchestrate_status_reads_persisted_state_with_sessions` template for new mesh/gossip status tests
