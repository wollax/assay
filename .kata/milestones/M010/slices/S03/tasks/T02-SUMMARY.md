---
id: T02
parent: S03
milestone: M010
provides:
  - Capability guard in run_mesh() that skips routing thread when supports_messaging is false
  - Capability guard in run_gossip() that skips PromptLayer injection and persist_knowledge_manifest when supports_gossip_manifest is false
  - tracing::warn! events with structured fields (capability, mode) emitted at each guard
  - Both T01 red-state degradation tests now pass green
  - All 1486 existing tests pass unchanged (zero regression)
key_files:
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/src/orchestrate/gossip.rs
key_decisions: []
patterns_established:
  - "Capability guard pattern: read capabilities() into a local bool before thread::scope, use the bool to conditionally skip feature-specific spawns/writes inside the scope"
  - "Guard placement: before the thread::scope block (mesh) or before the PromptLayer loop (gossip), not inside individual thread closures"
observability_surfaces:
  - "tracing::warn! with capability=\"messaging\", mode=\"mesh\" when mesh routing is skipped"
  - "tracing::warn! with capability=\"gossip_manifest\", mode=\"gossip\" when gossip manifest injection is skipped"
duration: 8min
verification_result: passed
completed_at: 2026-03-26T12:00:00Z
blocker_discovered: false
---

# T02: Capability guards in mesh routing and gossip manifest injection

**Added capability checks to run_mesh() and run_gossip() that skip messaging/manifest features when the backend doesn't support them, turning T01's red degradation tests green**

## What Happened

In `mesh.rs`, added a capability check before `thread::scope` that reads `config.backend.capabilities().supports_messaging`. When false, the routing thread spawn is skipped entirely (wrapped in `if supports_messaging { ... }`), but session worker spawns proceed normally. Inbox/outbox directories are still created.

In `gossip.rs`, added a capability check before the PromptLayer injection loop that reads `config.backend.capabilities().supports_gossip_manifest`. When false: (1) the gossip-knowledge-manifest PromptLayer is not pushed into session clones, (2) the initial `persist_knowledge_manifest` call is skipped, (3) the coordinator thread's per-completion and final `persist_knowledge_manifest` calls are skipped. The `supports_gossip_manifest` bool is captured before the `thread::scope` block and moved into the coordinator closure, so all three manifest write sites share the same guard.

Both changes emit a `tracing::warn!` with structured fields (`capability`, `mode`) when a capability is absent.

## Verification

| Check | Result |
|-------|--------|
| `cargo test -p assay-core --features orchestrate --test mesh_integration` | ✓ 3/3 passed (including `test_mesh_degrades_gracefully_without_messaging`) |
| `cargo test -p assay-core --features orchestrate --test gossip_integration` | ✓ 3/3 passed (including `test_gossip_degrades_gracefully_without_manifest`) |
| `cargo test -p assay-core --features orchestrate --test orchestrate_integration` | ✓ 5/5 passed |
| `cargo test -p assay-core --features orchestrate --test state_backend` | ✓ 19/19 passed |
| `just ready` | ✓ 1486 tests, fmt + lint + test + deny all green |

## Diagnostics

- Search for `warn!` events with `capability = "messaging"` or `capability = "gossip_manifest"` in structured logs/traces to identify when degradation is active
- Degradation integration tests (`test_mesh_degrades_gracefully_without_messaging`, `test_gossip_degrades_gracefully_without_manifest`) are the diagnostic surface — they define and verify the expected behavior

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/mesh.rs` — Added capability guard around routing thread spawn; warn event when messaging unsupported
- `crates/assay-core/src/orchestrate/gossip.rs` — Added capability guard around PromptLayer injection and all persist_knowledge_manifest calls; warn event when gossip manifest unsupported
