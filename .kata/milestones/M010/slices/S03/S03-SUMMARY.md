---
id: S03
parent: M010
milestone: M010
provides:
  - NoopBackend test helper in assay_core::state_backend (all capabilities false, all methods no-op)
  - NoopBackend exported from assay-core lib.rs alongside LocalFsBackend
  - capability check in run_mesh(): supports_messaging guards routing thread spawn
  - capability check in run_gossip(): supports_gossip_manifest guards PromptLayer injection and manifest writes
  - warn! events emitted on capability absence in both run_mesh() and run_gossip()
  - Integration test test_mesh_degrades_gracefully_without_messaging in mesh_integration.rs
  - Integration test test_gossip_degrades_gracefully_without_manifest in gossip_integration.rs
  - Three NoopBackend contract tests in state_backend.rs (capabilities, trait object, all methods succeed)
requires:
  - slice: S02
    provides: "Arc<dyn StateBackend> on OrchestratorConfig, backend.capabilities() callable"
affects:
  - S04
key_files:
  - crates/assay-core/src/state_backend.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/src/orchestrate/gossip.rs
  - crates/assay-core/tests/state_backend.rs
  - crates/assay-core/tests/mesh_integration.rs
  - crates/assay-core/tests/gossip_integration.rs
key_decisions:
  - NoopBackend as test helper rather than production type — proves degradation paths in isolation without real filesystem or remote backend
  - Capability check before routing thread spawn (not inside thread) — avoids spawning a thread that does nothing
  - Final manifest write in Disconnected arm also guarded by gossip_manifest_supported — necessary because this write is separate from the Ok(completion) arm
patterns_established:
  - Capability guard pattern: if !config.backend.capabilities().supports_X { tracing::warn!(...); skip feature }
  - NoopBackend for degradation test isolation
observability_surfaces:
  - warn! events with structured context when supports_messaging or supports_gossip_manifest is false
  - test_mesh_degrades/test_gossip_degrades pass with NoopBackend prove degradation is observable at test level
drill_down_paths:
  - .kata/milestones/M010/slices/S03/tasks/T01-PLAN.md
  - .kata/milestones/M010/slices/S03/tasks/T02-PLAN.md
duration: ~25min
verification_result: passed
completed_at: 2026-03-26
---

# S03: CapabilitySet degradation paths

**`NoopBackend` test helper + capability checks in `run_mesh()`/`run_gossip()` with `warn!` degradation events + two integration tests proving graceful behavior — R074 validated, all 1488 tests pass, `just ready` green.**

## What Happened

**T01** (combined with T02 in single pass) added `NoopBackend` to `assay_core::state_backend` — a zero-cost test helper implementing `StateBackend` with all capabilities false and all methods returning `Ok(())` / `Ok(None)` / `Ok(vec![])`. `NoopBackend` is exported from `assay-core`'s lib.rs alongside `LocalFsBackend`.

Capability checks were added to `run_mesh()` and `run_gossip()`:
- `run_mesh()`: checks `config.backend.capabilities().supports_messaging` before spawning the routing thread. When false, emits `warn!` explaining messaging is unavailable, skips the routing thread entirely. Sessions still execute in parallel.
- `run_gossip()`: checks `config.backend.capabilities().supports_gossip_manifest` before injecting the knowledge-manifest `PromptLayer` and before the initial/updated/final manifest writes. When false, emits `warn!` explaining manifest sharing is unavailable, skips all three write paths. Sessions still execute in parallel.

Three NoopBackend contract tests were added to `state_backend.rs`: `test_noop_backend_capabilities_all_false`, `test_noop_backend_as_trait_object`, `test_noop_backend_all_methods_succeed`.

Two integration tests were added: `test_mesh_degrades_gracefully_without_messaging` (NoopBackend → 2 sessions complete, no panic, outcomes are Completed) and `test_gossip_degrades_gracefully_without_manifest` (NoopBackend → 2 sessions complete, no gossip-knowledge-manifest PromptLayer injected, no knowledge.json written, no panic).

## Verification

- `cargo test -p assay-core --features orchestrate --test mesh_integration` — 3/3 pass (including new degradation test) ✅
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — 3/3 pass (including new degradation test) ✅
- `cargo test -p assay-core --features orchestrate --test state_backend` — 19/19 pass (including 3 new NoopBackend tests) ✅
- `cargo test --workspace` — 1488 tests, 0 failures ✅
- `just ready` — fmt + lint + test + deny all green ✅
- `grep -n "NoopBackend" crates/assay-core/src/state_backend.rs` — type present ✅

## Requirements Advanced

- R074 — CapabilitySet and graceful degradation: fully proven. Both degradation paths tested with NoopBackend. Each produces warn! event and no panic.

## Deviations

- The final manifest write in the `Disconnected` arm of the coordinator loop also needed to be guarded (it was initially missed). Added `if gossip_manifest_supported { ... }` around it after the test caught the file being written.
- `gossip_manifest_supported` is a `bool` (Copy) captured by value in the coordinator thread closure — no explicit variable binding needed before `scope.spawn`.

## Known Limitations

- NoopBackend is in the public API (exported from lib.rs) for test use. It is not intended for production use but there's no enforcement preventing it.

## Follow-ups

- S04: smelt-agent plugin documentation consuming the backend-aware API surface.

## Files Created/Modified

- `crates/assay-core/src/state_backend.rs` — added NoopBackend struct and StateBackend impl
- `crates/assay-core/src/lib.rs` — added NoopBackend to re-exports
- `crates/assay-core/src/orchestrate/mesh.rs` — capability check + routing thread conditional spawn
- `crates/assay-core/src/orchestrate/gossip.rs` — capability check + PromptLayer injection + manifest writes guarded
- `crates/assay-core/tests/state_backend.rs` — 3 new NoopBackend contract tests
- `crates/assay-core/tests/mesh_integration.rs` — added test_mesh_degrades_gracefully_without_messaging + NoopBackend import
- `crates/assay-core/tests/gossip_integration.rs` — added test_gossip_degrades_gracefully_without_manifest + NoopBackend import + walkdir_json helper
