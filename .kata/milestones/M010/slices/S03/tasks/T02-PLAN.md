---
estimated_steps: 5
estimated_files: 2
---

# T02: Capability guards in mesh routing and gossip manifest injection

**Slice:** S03 — CapabilitySet degradation paths
**Milestone:** M010

## Description

Add capability checks to `run_mesh()` and `run_gossip()` so that optional backend features are skipped gracefully when the backend doesn't support them. This turns the red-state integration tests from T01 green and validates R074.

## Steps

1. In `crates/assay-core/src/orchestrate/mesh.rs`, inside `run_mesh()`, before the `std::thread::scope` block, read `config.backend.capabilities()` into a local variable. Before spawning the routing thread (the first `scope.spawn` that polls outboxes and moves messages), check `capabilities.supports_messaging`. If false, emit `tracing::warn!(capability = "messaging", mode = "mesh", "backend does not support messaging — skipping mesh routing thread")` and do NOT spawn the routing thread. The session worker spawns that follow remain unchanged — sessions still launch and run normally. Inbox/outbox directories are still created (the roster PromptLayer references them and sessions may check for their existence).

2. In `crates/assay-core/src/orchestrate/gossip.rs`, inside `run_gossip()`, before the PromptLayer injection loop, read `config.backend.capabilities()` into a local variable. Check `capabilities.supports_gossip_manifest`. If false: (a) emit `tracing::warn!(capability = "gossip_manifest", mode = "gossip", "backend does not support gossip manifest — skipping knowledge manifest injection")`, (b) skip the PromptLayer push in the session clone loop (still build cloned_sessions but without the gossip-knowledge-manifest layer), (c) skip the initial `persist_knowledge_manifest` call, (d) in the coordinator thread, skip the per-completion and final `persist_knowledge_manifest` calls (guard with the same flag — pass the bool into the coordinator closure).

3. Run `cargo test -p assay-core --features orchestrate --test mesh_integration` — all tests including `test_mesh_degrades_gracefully_without_messaging` should pass.

4. Run `cargo test -p assay-core --features orchestrate --test gossip_integration` — all tests including `test_gossip_degrades_gracefully_without_manifest` should pass.

5. Run `just ready` — all 1481+ tests pass, fmt + lint + test + deny all green.

## Must-Haves

- [ ] `run_mesh()` checks `config.backend.capabilities().supports_messaging` before spawning the routing thread
- [ ] When `supports_messaging` is false, the routing thread is not spawned but sessions still execute normally
- [ ] A `tracing::warn!` is emitted when mesh messaging is skipped, with structured fields
- [ ] `run_gossip()` checks `config.backend.capabilities().supports_gossip_manifest` before PromptLayer injection
- [ ] When `supports_gossip_manifest` is false, no "gossip-knowledge-manifest" PromptLayer is injected and no `persist_knowledge_manifest` calls are made
- [ ] A `tracing::warn!` is emitted when gossip manifest is skipped, with structured fields
- [ ] `test_mesh_degrades_gracefully_without_messaging` passes
- [ ] `test_gossip_degrades_gracefully_without_manifest` passes
- [ ] All existing mesh/gossip/orchestrate/state_backend/pipeline tests pass unchanged
- [ ] `just ready` green

## Verification

- `cargo test -p assay-core --features orchestrate --test mesh_integration` — all pass (including degradation test)
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — all pass (including degradation test)
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — all pass (regression check)
- `cargo test -p assay-core --features orchestrate --test state_backend` — all pass (regression check)
- `cargo test --workspace` — all 1481+ pass
- `just ready` — green (fmt + lint + test + deny)

## Observability Impact

- Signals added/changed: `tracing::warn!` events emitted at mesh routing guard and gossip manifest guard with structured fields (`capability`, `mode`) — machine-readable for future tracing analysis
- How a future agent inspects this: search for `warn!` events with `capability = "messaging"` or `capability = "gossip_manifest"` in structured logs/traces
- Failure state exposed: if a capability is absent, the warn event names the capability and the mode, enabling the user/agent to understand why messaging or gossip manifest features are inactive

## Inputs

- `crates/assay-core/src/orchestrate/mesh.rs` — `run_mesh()` function, routing thread spawn at line ~222
- `crates/assay-core/src/orchestrate/gossip.rs` — `run_gossip()` function, PromptLayer injection at line ~148, `persist_knowledge_manifest` calls at lines ~165, ~267, ~356
- T01 output: `NoopBackend` available, degradation tests written and waiting to turn green
- S02 summary: `config.backend` is `Arc<dyn StateBackend>` — `capabilities()` callable via the trait method

## Expected Output

- `crates/assay-core/src/orchestrate/mesh.rs` — capability guard added around routing thread spawn
- `crates/assay-core/src/orchestrate/gossip.rs` — capability guard added around PromptLayer injection and manifest persistence
