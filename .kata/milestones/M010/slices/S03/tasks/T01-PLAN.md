---
estimated_steps: 4
estimated_files: 4
---

# T01: NoopBackend and capability guard tests (red state)

**Slice:** S03 — CapabilitySet degradation paths
**Milestone:** M010

## Description

Create the `NoopBackend` test helper struct in `state_backend.rs` and write three new tests: a contract test verifying NoopBackend's capabilities and method behavior, and two integration tests (one mesh, one gossip) that drive `run_mesh()` / `run_gossip()` with a NoopBackend-configured OrchestratorConfig. The integration tests define the expected degradation behavior and will be in red state (failing) until T02 adds the production capability guards.

## Steps

1. Add `NoopBackend` struct to `crates/assay-core/src/state_backend.rs`. It implements `StateBackend` with: `capabilities() → CapabilitySet::none()`, `push_session_event → Ok(())`, `read_run_state → Ok(None)`, `send_message → Ok(())`, `poll_inbox → Ok(vec![])`, `annotate_run → Ok(())`, `save_checkpoint_summary → Ok(())`. Mark it `pub` so tests in other modules can construct it.

2. Write `test_noop_backend_capabilities` in `crates/assay-core/tests/state_backend.rs`. Assert `NoopBackend.capabilities() == CapabilitySet::none()`. Assert all methods return `Ok`. This test should pass immediately (green).

3. Write `test_mesh_degrades_gracefully_without_messaging` in `crates/assay-core/tests/mesh_integration.rs`. Build an `OrchestratorConfig` with `backend: Arc::new(NoopBackend)`. Build a 2-session mesh manifest with a writer and reader. Use a simple success runner (no actual message writing). Call `run_mesh()`. Assert: result is `Ok`, `result.status.sessions` has correct count, mesh_status `messages_routed == 0`, all sessions are in a terminal state. Note: this test will FAIL in red state because `run_mesh()` currently doesn't check capabilities before routing — the routing thread tries to operate on a backend that doesn't support messaging, but since NoopBackend's methods return Ok, the issue is actually that the test should pass once the routing thread simply doesn't crash. The real assertion is that routing is skipped.

4. Write `test_gossip_degrades_gracefully_without_manifest` in `crates/assay-core/tests/gossip_integration.rs`. Build an `OrchestratorConfig` with `backend: Arc::new(NoopBackend)`. Build a 2-session gossip manifest. Use a runner that records each session's prompt_layers. Call `run_gossip()`. Assert: result is `Ok`, sessions have correct count, no session received a "gossip-knowledge-manifest" PromptLayer, gossip_status `sessions_synthesized` matches session count. Note: this test will FAIL in red state because `run_gossip()` currently injects the PromptLayer unconditionally and calls `persist_knowledge_manifest` which will error when NoopBackend's gossip_manifest capability is false.

## Must-Haves

- [ ] `NoopBackend` struct exists in `crates/assay-core/src/state_backend.rs` with `pub` visibility
- [ ] `NoopBackend.capabilities()` returns `CapabilitySet::none()` (all false)
- [ ] All `NoopBackend` methods return success values (Ok(()) / Ok(None) / Ok(vec![]))
- [ ] `test_noop_backend_capabilities` passes in `state_backend.rs` test file
- [ ] `test_mesh_degrades_gracefully_without_messaging` exists and compiles in `mesh_integration.rs`
- [ ] `test_gossip_degrades_gracefully_without_manifest` exists and compiles in `gossip_integration.rs`
- [ ] All existing state_backend, mesh, gossip, orchestrate tests still pass

## Verification

- `cargo test -p assay-core --features orchestrate --test state_backend test_noop` — NoopBackend contract test passes
- `cargo test -p assay-core --features orchestrate --test state_backend` — all existing tests pass
- `cargo test -p assay-core --features orchestrate --test mesh_integration test_mesh_mode_completed` — existing tests still pass
- `cargo test -p assay-core --features orchestrate --test gossip_integration test_gossip_mode` — existing tests still pass
- Degradation tests compile (may fail in red state — this is expected and documented)

## Observability Impact

- Signals added/changed: None in this task (tests only)
- How a future agent inspects this: `cargo test -p assay-core --features orchestrate --test state_backend test_noop` confirms NoopBackend contract
- Failure state exposed: None — NoopBackend's methods always succeed; failure exposure comes from the production guards in T02

## Inputs

- `crates/assay-core/src/state_backend.rs` — existing `StateBackend` trait, `CapabilitySet`, `LocalFsBackend`
- `crates/assay-core/tests/mesh_integration.rs` — existing test helpers (`setup_temp_dir`, `make_pipeline_config`, `make_mesh_manifest`, `success_result`)
- `crates/assay-core/tests/gossip_integration.rs` — existing test helpers (same pattern)
- S01 summary: `CapabilitySet::none()` constructor exists and returns all-false
- S02 summary: `OrchestratorConfig.backend` is `Arc<dyn StateBackend>`, default uses `LocalFsBackend`

## Expected Output

- `crates/assay-core/src/state_backend.rs` — `NoopBackend` struct added at the end (pub, implements StateBackend)
- `crates/assay-core/tests/state_backend.rs` — `test_noop_backend_capabilities` test added
- `crates/assay-core/tests/mesh_integration.rs` — `test_mesh_degrades_gracefully_without_messaging` test added
- `crates/assay-core/tests/gossip_integration.rs` — `test_gossip_degrades_gracefully_without_manifest` test added
