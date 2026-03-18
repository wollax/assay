---
id: T02
parent: S04
milestone: M004
provides:
  - orchestrate_status_returns_mesh_status MCP test asserting mesh_status JSON round-trip
  - orchestrate_status_returns_gossip_status MCP test asserting gossip_status JSON round-trip
  - integration_modes.rs all-modes regression suite (DAG + Mesh + Gossip in one file)
key_files:
  - crates/assay-mcp/tests/mcp_handlers.rs
  - crates/assay-core/tests/integration_modes.rs
key_decisions:
  - DAG integration_modes test uses setup_git_repo() helper for consistency with orchestrate_integration.rs even though the mock runner doesn't perform git operations (run_orchestrated itself has no git dependencies)
  - Gossip assertion checks knowledge.entries.len() == 2 by deserializing knowledge.json into KnowledgeManifest
patterns_established:
  - All three mode executors (run_orchestrated, run_mesh, run_gossip) verified with the same success_result mock runner pattern
  - MCP status round-trip tests: write realistic OrchestratorStatus to state.json, call orchestrate_status(), assert response JSON fields
observability_surfaces:
  - cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status — targeted verification of mesh_status JSON surface
  - cargo test -p assay-mcp -- orchestrate_status_returns_gossip_status — targeted verification of gossip_status JSON surface
  - cargo test -p assay-core --features orchestrate --test integration_modes -- --nocapture — verbose all-modes sweep
duration: ~10min
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T02: Add MCP Status Tests (Mesh/Gossip) and All-Modes Core Integration Test

**Added 2 MCP handler tests proving mesh_status/gossip_status survive the state.json round-trip, and created integration_modes.rs with a 3-test all-modes regression suite (DAG + Mesh + Gossip).**

## What Happened

Two coverage gaps from the T01 plan were closed:

1. **MCP observability tests** — Added `orchestrate_status_returns_mesh_status` and `orchestrate_status_returns_gossip_status` to `mcp_handlers.rs`. Each test builds a realistic `OrchestratorStatus` with the relevant mode-specific status field populated, serializes it to `state.json` in a temp dir, calls `server.orchestrate_status()`, and asserts the response JSON contains the expected nested field values. The mesh test verifies `messages_routed == 3` and `members[0].state == "completed"`; the gossip test verifies `sessions_synthesized == 2` and `coordinator_rounds == 4`. Both also assert the absent sibling field is null/missing.

2. **All-modes integration suite** — Created `crates/assay-core/tests/integration_modes.rs` with `#![cfg(feature = "orchestrate")]` guard. The file defines all helpers locally (no imports from other test files): `setup_temp_dir()`, `setup_git_repo()`, `make_pipeline_config()`, `make_dag_manifest()`, `make_mesh_manifest()`, `make_gossip_manifest()`, and `success_result()`. Three tests exercise each mode with a 2-session manifest and mock success runners, asserting 2 `Completed` outcomes and verifying mode-specific side effects (state.json for Mesh, knowledge.json with 2 entries for Gossip).

## Verification

```
cargo test -p assay-mcp -- orchestrate_status_returns_mesh_status orchestrate_status_returns_gossip_status
# → 2 passed

cargo test -p assay-core --features orchestrate --test integration_modes
# → 3 passed (dag, mesh, gossip)

cargo test -p assay-mcp
# → 31 passed (29 existing + 2 new, 0 failures)

cargo test --workspace --features "assay-core/orchestrate"
# → 1271 total tests, all passing (≥1270 slice criterion met)
```

## Diagnostics

- `cargo test -p assay-mcp -- orchestrate_status_returns_mesh` — fails immediately if `mesh_status` removed from `OrchestratorStatus` or serde attributes change
- `cargo test -p assay-core --features orchestrate --test integration_modes -- --nocapture` — verbose output shows which mode is under test
- All three mode dispatchers (run_orchestrated, run_mesh, run_gossip) now have a single regression target in integration_modes.rs

## Deviations

- The DAG test uses `setup_git_repo()` for consistency, though the mock runner doesn't need git (run_orchestrated has no git dependency itself). The task plan suggested this approach explicitly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-mcp/tests/mcp_handlers.rs` — Added `orchestrate_status_returns_mesh_status` and `orchestrate_status_returns_gossip_status` tests at end of orchestrate_status test block
- `crates/assay-core/tests/integration_modes.rs` — New file: `#![cfg(feature = "orchestrate")]` guard, all helpers defined locally, 3 tests covering DAG/Mesh/Gossip dispatch
