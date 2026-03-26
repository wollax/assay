---
id: T01
parent: S03
milestone: M010
provides:
  - NoopBackend struct with all capabilities disabled
  - NoopBackend contract test (capabilities + all methods return Ok)
  - NoopBackend trait-object test (Arc<dyn StateBackend>)
  - Mesh degradation test (test_mesh_degrades_gracefully_without_messaging)
  - Gossip degradation test (test_gossip_degrades_gracefully_without_manifest)
key_files:
  - crates/assay-core/src/state_backend.rs
  - crates/assay-core/tests/state_backend.rs
  - crates/assay-core/tests/mesh_integration.rs
  - crates/assay-core/tests/gossip_integration.rs
key_decisions:
  - "NoopBackend returns Ok for all methods including send_message/annotate_run — it does not error even when capability is false, unlike LocalFsBackend's contract guidance. This is intentional: NoopBackend is a test helper for degradation paths, not a production backend."
patterns_established:
  - "Degradation tests use OrchestratorConfig { backend: Arc::new(NoopBackend), ..Default::default() } to inject a capability-less backend"
  - "Gossip degradation test captures PromptLayer injection via Arc<Mutex<Vec<String>>> shared between runner closure and test assertions"
observability_surfaces:
  - none (tests only — no runtime signals added)
duration: 12min
verification_result: passed
completed_at: 2026-03-26T12:00:00Z
blocker_discovered: false
---

# T01: NoopBackend and capability guard tests (red state)

**Added `NoopBackend` test helper and two degradation integration tests — mesh passes green, gossip fails red as expected**

## What Happened

Added `NoopBackend` struct to `state_backend.rs` implementing `StateBackend` with `CapabilitySet::none()` and all methods returning success values. Exported it from the crate via `lib.rs`.

Wrote three contract tests in `state_backend.rs`: capabilities assertion, all-methods-return-Ok verification, and Arc trait-object usage.

Wrote `test_mesh_degrades_gracefully_without_messaging` in `mesh_integration.rs`. This test passes green because NoopBackend's `send_message`/`poll_inbox` silently succeed (no-op), so the mesh routing thread doesn't crash — it just finds no messages to route.

Wrote `test_gossip_degrades_gracefully_without_manifest` in `gossip_integration.rs`. This test fails red because `run_gossip()` unconditionally injects the `gossip-knowledge-manifest` PromptLayer without checking `capabilities().supports_gossip_manifest`. The test asserts no session receives this layer, catching the missing guard. T02 will add the production capability check to make this test pass.

## Verification

- `cargo test -p assay-core --features orchestrate --test state_backend test_noop` — 3 passed (NoopBackend contract)
- `cargo test -p assay-core --features orchestrate --test state_backend` — 19 passed (all existing + new)
- `cargo test -p assay-core --features orchestrate --test mesh_integration test_mesh_mode_completed` — passed (existing test)
- `cargo test -p assay-core --features orchestrate --test mesh_integration test_mesh_degrades` — passed (green — routing no-ops)
- `cargo test -p assay-core --features orchestrate --test gossip_integration test_gossip_mode` — 2 passed (existing tests)
- `cargo test -p assay-core --features orchestrate --test gossip_integration test_gossip_degrades` — FAILED (expected red state — PromptLayer injected unconditionally)

### Slice Verification Status
- `test_mesh_degrades` — ✓ passes (will still pass after T02)
- `test_gossip_degrades` — ✗ fails (expected, T02 fixes)
- All existing mesh/gossip/state_backend tests — ✓ pass (zero regression)
- `grep -rn "NoopBackend" crates/assay-core/src/state_backend.rs` — ✓ type exists
- `just ready` — not run (gossip degradation test intentionally fails; will be green after T02)

## Diagnostics

`cargo test -p assay-core --features orchestrate --test state_backend test_noop` confirms NoopBackend contract. Degradation tests are the diagnostic surface — they define the expected behavior and will go green when T02 adds capability guards.

## Deviations

- Task plan predicted mesh degradation test would fail, but it passes green because NoopBackend's methods silently succeed and the routing thread tolerates empty inboxes. The gossip test correctly fails as predicted.
- Used `OrchestratorResult.outcomes` for assertions instead of reading `state.json` from disk, since NoopBackend's `push_session_event` is a no-op and writes nothing.
- Fixed `TeamCheckpoint` construction to match current struct fields (version/session_id/project/timestamp/trigger/agents instead of team_name/generated_at).

## Known Issues

- Gossip degradation test fails until T02 adds the `supports_gossip_manifest` capability guard to `run_gossip()`.

## Files Created/Modified

- `crates/assay-core/src/state_backend.rs` — Added `NoopBackend` struct implementing `StateBackend` with all-disabled capabilities
- `crates/assay-core/src/lib.rs` — Added `NoopBackend` to public exports
- `crates/assay-core/tests/state_backend.rs` — Added 3 NoopBackend contract tests
- `crates/assay-core/tests/mesh_integration.rs` — Added `test_mesh_degrades_gracefully_without_messaging`
- `crates/assay-core/tests/gossip_integration.rs` — Added `test_gossip_degrades_gracefully_without_manifest`
