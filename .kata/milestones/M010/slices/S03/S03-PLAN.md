# S03: CapabilitySet degradation paths

**Goal:** The orchestrator checks `backend.capabilities()` before exercising optional backend features (messaging, gossip manifest) and degrades gracefully when those capabilities are absent — emitting `warn!` events and continuing without the feature, never panicking or erroring out.
**Demo:** A `NoopBackend` with all capabilities false drives `run_mesh()` and `run_gossip()` to completion without errors. Mesh skips message routing; Gossip omits the knowledge-manifest PromptLayer. Logs show clear warnings explaining why the features were skipped.

## Must-Haves

- `NoopBackend` test helper exists in `state_backend.rs` with `capabilities() → CapabilitySet::none()` and all methods returning `Ok(())` / `Ok(None)` / `Ok(vec![])`
- `run_mesh()` checks `config.backend.capabilities().supports_messaging` before starting the routing thread; when false, routing is skipped entirely and a `warn!` event is emitted with the reason
- `run_gossip()` checks `config.backend.capabilities().supports_gossip_manifest` before injecting the knowledge-manifest PromptLayer and before writing the initial/updated manifest; when false, the PromptLayer is omitted and manifest writes are skipped, with a `warn!` event explaining why
- Integration test `test_mesh_degrades_gracefully_without_messaging` proves `run_mesh()` completes with `NoopBackend`, sessions still execute, mesh_status has `messages_routed == 0`, and no error is returned
- Integration test `test_gossip_degrades_gracefully_without_manifest` proves `run_gossip()` completes with `NoopBackend`, sessions still execute, no "gossip-knowledge-manifest" PromptLayer is injected into sessions, and no error is returned
- All existing mesh/gossip/orchestrate tests pass unchanged (zero regression)
- R074 is proven: each degradation path has a test, produces a clear warning, and does not panic

## Proof Level

- This slice proves: integration
- Real runtime required: no (mock runners, no real agents)
- Human/UAT required: no

## Verification

- `cargo test -p assay-core --features orchestrate --test mesh_integration test_mesh_degrades` — degradation test passes
- `cargo test -p assay-core --features orchestrate --test gossip_integration test_gossip_degrades` — degradation test passes
- `cargo test -p assay-core --features orchestrate --test mesh_integration` — all mesh tests pass (zero regression)
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — all gossip tests pass (zero regression)
- `cargo test -p assay-core --features orchestrate --test state_backend` — all state_backend tests pass (NoopBackend contract)
- `just ready` — green (fmt + lint + test + deny)
- `grep -rn "NoopBackend" crates/assay-core/src/state_backend.rs` — type exists

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` events emitted at the orchestrator level when a capability is absent — machine-readable via structured tracing fields (`capability = "messaging"`, `mode = "mesh"` / `mode = "gossip"`)
- Inspection surfaces: integration tests assert on `OrchestratorResult` fields (mesh_status.messages_routed, session count, completion states)
- Failure visibility: if a capability check is accidentally bypassed, the `NoopBackend` methods return `Ok(())` silently — but the test assertions on `PromptLayer` absence and `messages_routed == 0` catch the omission
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `OrchestratorConfig.backend` (Arc<dyn StateBackend>) from S02; `CapabilitySet` flags from S01; `run_mesh()` and `run_gossip()` function bodies from M004
- New wiring introduced in this slice: capability guards inserted at two points in production code (mesh routing thread setup, gossip PromptLayer injection); `NoopBackend` test helper for degradation-pattern testing
- What remains before the milestone is truly usable end-to-end: S04 (smelt-agent plugin — documentation only, no code changes)

## Tasks

- [x] **T01: NoopBackend and capability guard tests (red state)** `est:20m`
  - Why: Establishes the test contracts that define correct degradation behavior before modifying production code. `NoopBackend` is also the reusable test helper for any future backend-degradation testing.
  - Files: `crates/assay-core/src/state_backend.rs`, `crates/assay-core/tests/state_backend.rs`, `crates/assay-core/tests/mesh_integration.rs`, `crates/assay-core/tests/gossip_integration.rs`
  - Do: Add `NoopBackend` struct (all capabilities false, all methods no-op) to `state_backend.rs`. Write `test_noop_backend_capabilities` contract test. Write `test_mesh_degrades_gracefully_without_messaging` in `mesh_integration.rs` that runs `run_mesh()` with a `NoopBackend` config and asserts sessions complete, `messages_routed == 0`, no error. Write `test_gossip_degrades_gracefully_without_manifest` in `gossip_integration.rs` that runs `run_gossip()` with a `NoopBackend` config and asserts sessions complete, no "gossip-knowledge-manifest" PromptLayer in cloned sessions, no error. Tests will fail (red state) because production code doesn't guard capabilities yet.
  - Verify: `cargo test -p assay-core --features orchestrate --test state_backend test_noop` — passes (NoopBackend contract); mesh/gossip degradation tests compile but fail (expected red state)
  - Done when: `NoopBackend` type exists, 3 new tests compile, NoopBackend contract test passes, mesh/gossip tests fail with expected behavior (routing thread panics or PromptLayer is injected when it shouldn't be)

- [x] **T02: Capability guards in mesh routing and gossip manifest injection** `est:20m`
  - Why: Implements the actual degradation behavior — capability checks in `run_mesh()` and `run_gossip()` that skip messaging/manifest features when the backend doesn't support them. Turns T01's red tests green.
  - Files: `crates/assay-core/src/orchestrate/mesh.rs`, `crates/assay-core/src/orchestrate/gossip.rs`
  - Do: In `run_mesh()`, before the `thread::scope` block where the routing thread is spawned, check `config.backend.capabilities().supports_messaging`. If false, emit `warn!(capability = "messaging", mode = "mesh", "backend does not support messaging — skipping mesh routing thread")` and skip spawning the routing thread. The session workers still launch and execute normally. Inbox/outbox directories are still created (sessions may reference them in their roster, and directory absence would cause errors). In `run_gossip()`, before the PromptLayer injection loop, check `config.backend.capabilities().supports_gossip_manifest`. If false, emit `warn!(capability = "gossip_manifest", mode = "gossip", "backend does not support gossip manifest — skipping knowledge manifest injection")` and skip: (1) the PromptLayer push, (2) the `persist_knowledge_manifest` calls (initial write, coordinator updates, final write). Sessions still launch and run. The coordinator thread still processes completions for status tracking; it just skips manifest writes.
  - Verify: `cargo test -p assay-core --features orchestrate --test mesh_integration` — all tests pass including degradation; `cargo test -p assay-core --features orchestrate --test gossip_integration` — all tests pass including degradation; `just ready` — green
  - Done when: Both degradation tests pass, all existing mesh/gossip/orchestrate/state_backend tests pass unchanged, `just ready` green

## Files Likely Touched

- `crates/assay-core/src/state_backend.rs`
- `crates/assay-core/tests/state_backend.rs`
- `crates/assay-core/src/orchestrate/mesh.rs`
- `crates/assay-core/src/orchestrate/gossip.rs`
- `crates/assay-core/tests/mesh_integration.rs`
- `crates/assay-core/tests/gossip_integration.rs`
