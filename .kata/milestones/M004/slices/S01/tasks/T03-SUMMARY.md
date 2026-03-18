---
id: T03
parent: S01
milestone: M004
provides:
  - run_mesh() stub in assay-core::orchestrate::mesh — emits tracing::warn! per session with depends_on, returns OrchestratorResult with zero outcomes
  - run_gossip() stub in assay-core::orchestrate::gossip — identical structure, Gossip warning message
  - pub mod mesh; pub mod gossip; declared in assay-core::orchestrate::mod.rs
  - CLI execute() mode dispatch: match manifest.mode { Mesh => execute_mesh, Gossip => execute_gossip, Dag => {} } before needs_orchestration check
  - MCP orchestrate_run multi-session guard conditioned on mode == OrchestratorMode::Dag
  - MCP Mesh/Gossip routing branches via spawn_blocking to stubs
  - Default impl for RunManifest in assay-types (mode: Dag, sessions: vec![], no configs)
key_files:
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/src/orchestrate/gossip.rs
  - crates/assay-core/src/orchestrate/mod.rs
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-types/src/manifest.rs
key_decisions:
  - Added Default impl for RunManifest (Dag mode, empty sessions) to unblock test struct literals across the workspace — multiple crates build RunManifest in tests and the new deny_unknown_fields struct required all three new fields; Default avoids cascading explicit field additions in every test helper
patterns_established:
  - Mode dispatch in CLI: match on manifest.mode before needs_orchestration(); Dag arm falls through, Mesh/Gossip arms return early
  - MCP mode routing: match arm before the DAG spawn_blocking block; each mode calls stub via spawn_blocking and returns early
  - Stub executor signature: run_mesh/run_gossip accept (&RunManifest, &OrchestratorConfig, &PipelineConfig, &F) matching run_orchestrated; session_runner is unreachable! in stubs
observability_surfaces:
  - tracing::warn! per session with non-empty depends_on when mode is Mesh or Gossip — observable via RUST_LOG=warn
  - OrchestratorResult.run_id returned by stubs — can be passed to orchestrate_status for inspection
duration: ~1h
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Add run_mesh/run_gossip stubs, wire mod.rs, dispatch in CLI and MCP

**Created `run_mesh()` and `run_gossip()` stubs in `assay-core::orchestrate`, wired mode dispatch in both CLI `execute()` and MCP `orchestrate_run`, conditioned the MCP multi-session guard on `mode == Dag`, and fixed all downstream test struct literals by adding `Default` for `RunManifest`; `just ready` green with 0 warnings.**

## What Happened

Created `crates/assay-core/src/orchestrate/mesh.rs` and `gossip.rs` with identical structure: accept the same four-argument signature as `run_orchestrated()`, emit `tracing::warn!` per session with non-empty `depends_on`, return a valid `OrchestratorResult` with `Ulid::new()` run_id, zero outcomes, and `Duration::ZERO`. Declared both modules in `mod.rs`.

In `crates/assay-cli/src/commands/run.rs`: added `use assay_types::OrchestratorMode` import; added a `match manifest.mode { Mesh => ..., Gossip => ..., Dag => {} }` block before the `needs_orchestration()` check in `execute()`; added `execute_mesh()` and `execute_gossip()` private functions that call the stubs and return a minimal `OrchestrationResponse` with empty sessions and zero summary; added three mode-dispatch unit tests.

In `crates/assay-mcp/src/server.rs`: added `OrchestratorMode` to the `use assay_types` import; conditioned the multi-session guard on `manifest.mode == OrchestratorMode::Dag`; added Mesh and Gossip routing arms (before the DAG `spawn_blocking` block) that call the stubs via `spawn_blocking` and return a minimal `OrchestrateRunResponse`; added two async unit tests verifying the guard bypass.

**Deviation from plan:** The task plan did not mention that adding the three new `RunManifest` fields in T02 broke every struct literal test across the workspace (manifest.rs, dag.rs, executor.rs, pipeline.rs, orchestrate_integration.rs, assay-cli run.rs). These were silently broken because `just build` passes but `cargo test --no-run` does not. Fixed by adding `impl Default for RunManifest` in `assay-types/src/manifest.rs` (Dag mode, empty sessions, no configs) and updating the integration test helper in `orchestrate_integration.rs`.

## Verification

```
cargo test -p assay-core --features orchestrate   → ok. 5 passed (unit + integration)
cargo test -p assay-cli                           → ok. 30 passed (includes 3 new mode dispatch tests)
cargo test -p assay-mcp                           → ok. 112 + 29 passed (includes 2 new guard-bypass tests)
just ready                                        → fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓
```

## Diagnostics

- `RUST_LOG=warn cargo test` — shows `tracing::warn!` messages when a test exercises Mesh/Gossip sessions with `depends_on`
- Stubs return `OrchestratorResult` with a `run_id` (ULID); pass to `orchestrate_status` to inspect persisted state
- If a stub panics (unreachable! in session_runner), the `spawn_blocking` boundary surfaces it as `Err` at the call site

## Deviations

- Added `impl Default for RunManifest` to fix cascading test struct literal failures across the workspace. The task plan did not anticipate this breakage (T02 added `deny_unknown_fields` fields without updating all test helpers). This is additive and does not affect the serde contract (no `#[derive(Default)]`, no schema change).
- Added `make_manifest_with_mode()` helper in CLI tests instead of using `..Default::default()` in individual test bodies (cleaner).
- Fixed `orchestrate_integration.rs` make_manifest helper to include new `RunManifest` fields (missed by T02).

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/mesh.rs` — new: `run_mesh()` stub with warn loop and unit tests
- `crates/assay-core/src/orchestrate/gossip.rs` — new: `run_gossip()` stub with warn loop and unit tests
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod mesh; pub mod gossip;`
- `crates/assay-core/src/orchestrate/dag.rs` — fixed test `make_manifest` helper (..Default::default())
- `crates/assay-core/src/orchestrate/executor.rs` — fixed test `make_manifest` helper
- `crates/assay-core/src/manifest.rs` — fixed test struct literals (9 occurrences)
- `crates/assay-core/src/pipeline.rs` — fixed test `RunManifest` literal
- `crates/assay-core/tests/orchestrate_integration.rs` — fixed `make_manifest` helper
- `crates/assay-cli/src/commands/run.rs` — mode dispatch in execute(), execute_mesh/gossip stubs, 3 unit tests
- `crates/assay-mcp/src/server.rs` — OrchestratorMode import, guard conditioned on Dag, Mesh/Gossip routing, 2 unit tests
- `crates/assay-types/src/manifest.rs` — added `impl Default for RunManifest`
