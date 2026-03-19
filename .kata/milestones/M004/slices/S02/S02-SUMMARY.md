---
id: S02
parent: M004
milestone: M004
provides:
  - MeshMemberState, MeshMemberStatus, MeshStatus types in assay-types::orchestrate with schema snapshots locked
  - OrchestratorStatus.mesh_status optional field (backward-compatible, serde default)
  - persist_state pub(crate) in executor.rs for reuse by mesh.rs
  - Full run_mesh() implementation: parallel dispatch, roster PromptLayer injection, inbox/outbox dirs, routing thread, SWIM membership, state persistence
  - Integration tests: test_mesh_mode_message_routing and test_mesh_mode_completed_not_dead
requires:
  - slice: S01
    provides: OrchestratorMode::Mesh dispatch routing, MeshConfig type, run_mesh() stub signature
affects:
  - S03: GossipStatus pattern follows MeshStatus; run_gossip() follows run_mesh() threading model
  - S04: mesh_status field in OrchestratorStatus is the surface orchestrate_status will expose
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-state-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-status-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__mesh-status-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap
  - crates/assay-core/src/orchestrate/executor.rs
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-core/tests/mesh_integration.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
key_decisions:
  - D057: persist_state made pub(crate) in executor.rs for reuse by mesh.rs
  - D058: Mesh roster PromptLayer uses "Outbox: <path>" as machine-parseable line for session outbox discovery
  - Routing thread uses active_count AtomicUsize as sole termination signal; exits when count reaches 0
  - Bounded concurrency via (Mutex<usize>, Condvar) counting semaphore — same pattern as executor.rs DAG dispatch
  - MeshMemberState::Dead vs Completed distinguishes crash/pipeline error from normal session exit
patterns_established:
  - thread::scope with routing thread + N worker threads sharing Arc<AtomicUsize> active_count as termination signal
  - Best-effort persist_state inside worker after updating both Arc<Mutex<>> states (clone under lock, persist outside lock)
  - MeshMemberState transitions: Alive → Running → Completed (Ok) or Dead (Err/panic)
  - Integration tests for mesh/gossip use bare tempdir (no git init) — only .assay/orchestrator/ structure needed
observability_surfaces:
  - "cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status — members list with per-session state + messages_routed"
  - "RUST_LOG=assay_core=debug cargo test -- mesh --nocapture — shows per-message routing events (from, to, filename)"
  - MeshMemberState::Dead in state.json indicates crash; Completed indicates normal exit
  - tracing::info! at session launch, tracing::debug! per routed message, tracing::warn! for unrecognized targets and depends_on
drill_down_paths:
  - .kata/milestones/M004/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M004/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M004/slices/S02/tasks/T03-SUMMARY.md
  - .kata/milestones/M004/slices/S02/tasks/T04-SUMMARY.md
duration: ~80min total (T01 ~20m, T02 ~15m, T03 ~45m, T04 <5m)
verification_result: passed
completed_at: 2026-03-18
---

# S02: Mesh Mode

**Full mesh executor ships: parallel session dispatch with roster injection, file-based outbox→inbox message routing, SWIM-inspired membership tracking, and state.json persistence — proven by two integration tests with zero existing tests broken.**

## What Happened

S02 replaced the `run_mesh()` stub from S01 with a complete implementation across four tasks:

**T01** established the type contract by adding `MeshMemberState` (Alive/Suspect/Dead/Completed), `MeshMemberStatus`, and `MeshStatus` to `assay-types::orchestrate`, extended `OrchestratorStatus` with an optional `mesh_status` field, and locked all four schema snapshots. `persist_state` was made `pub(crate)` in `executor.rs` so mesh.rs could reuse the atomic write helper. All 254 assay-types tests passed after regenerating snapshots with `INSTA_UPDATE=always`.

**T02** wrote the two integration tests as failing contracts before any implementation: `test_mesh_mode_message_routing` (writer session creates outbox file targeting reader, asserts file arrives in reader inbox with `messages_routed >= 1`) and `test_mesh_mode_completed_not_dead` (both sessions complete normally, all members show `Completed` state). Both failed against the stub with clear assertion messages — exactly as intended.

**T03** replaced the stub with the full implementation: inbox/outbox directories per session, roster `PromptLayer` (kind: System, priority: -5, containing "Outbox: <path>" and "Peer: <name> Inbox: <path>" for each session), `thread::scope` with a routing thread (polls outbox subdirs every 50ms, renames files to target inboxes, increments `messages_routed`, exits when `active_count == 0`) and N worker threads (bounded concurrency via `(Mutex<usize>, Condvar)` semaphore, `panic::catch_unwind` around runner calls, writes `completed` sentinel, updates `MeshMemberState`, best-effort persists state). Also fixed two `OrchestratorStatus` construction sites in `assay-mcp` that were missing `mesh_status: None`.

**T04** ran `just ready` — it passed on the first attempt with no fixes needed. All snapshots were already stable.

## Verification

- `cargo test -p assay-types --features orchestrate` — 254 tests pass; 4 mesh-related snapshots locked
- `cargo test -p assay-core --features orchestrate -- mesh` — 6 tests pass (4 unit + 2 integration)
- `cargo test -p assay-core --features orchestrate` — 777 tests pass (770 unit + 2 mesh integration + 5 orchestrate integration)
- `just ready` — fmt ✓ lint ✓ (0 warnings) test ✓ deny ✓; all 1230+ tests pass

## Requirements Advanced

- R035 (Mesh mode execution) — proved: parallel launch with roster PromptLayer injection; all sessions start immediately without DAG ordering; `depends_on` sessions emit warn and continue
- R036 (Mesh peer messaging) — proved: file-based outbox→inbox routing works; integration test moves a real file; SWIM-inspired membership tracks Alive/Completed/Dead states in state.json; `messages_routed` counter is accurate

## Requirements Validated

- R035 — `test_mesh_mode_message_routing` and `test_mesh_mode_completed_not_dead` prove the full mesh contract with real filesystem operations; state.json persists correct membership states; schema snapshots locked
- R036 — routing thread correctly polls `outbox/<target>/` dirs and moves files to `<target>/inbox/`; `messages_routed` increments per file; unrecognized targets emit tracing::warn; membership distinguishes Completed from Dead

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- `assay-mcp/src/server.rs` and `assay-mcp/tests/mcp_handlers.rs` had `OrchestratorStatus` construction sites that needed `mesh_status: None` added — minor collateral work not in the task plan, resolved in T03 without issue
- T03 replaced the stub's unit tests (which tested the stub's no-op behavior) with implementation-accurate tests; no test count was lost (4 new unit tests vs 2 old stub tests)

## Known Limitations

- Heartbeat files (for alive/suspect transitions) are not yet polled — sessions start `Alive` and transition directly to `Completed` or `Dead` on exit; the `Suspect` state is defined but not reachable yet. This is intentional: S02 proves the routing and completion mechanics; heartbeat polling is a correctness refinement deferred to S04 or post-M004
- `last_heartbeat_at` is always `None` in current state.json (no heartbeat writes yet); the field schema is locked and the field is backward-compatible
- S04 is required before `mesh_status` is surfaced in the `orchestrate_status` MCP tool response — S02 writes it to state.json but S04 reads and returns it

## Follow-ups

- S04 must wire `mesh_status` from state.json into the `orchestrate_status` MCP tool response
- S04 should add heartbeat polling to enable `Alive → Suspect → Dead` transitions for sessions that crash silently
- Gossip executor (S03) should follow the same `thread::scope` + `Arc<AtomicUsize> active_count` pattern

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — Added MeshMemberState, MeshMemberStatus, MeshStatus; extended OrchestratorStatus with mesh_status; 3 inventory entries; tests
- `crates/assay-types/src/lib.rs` — Re-exported MeshMemberState, MeshMemberStatus, MeshStatus under #[cfg(feature = "orchestrate")]
- `crates/assay-types/tests/schema_snapshots.rs` — Added 3 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-state-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-status-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-status-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — Updated with mesh_status field, locked
- `crates/assay-core/src/orchestrate/executor.rs` — persist_state now pub(crate); 3 OrchestratorStatus construction sites updated with mesh_status: None
- `crates/assay-core/src/orchestrate/mesh.rs` — Full implementation replacing stub; 4 unit tests
- `crates/assay-core/tests/mesh_integration.rs` — New; 2 integration tests
- `crates/assay-mcp/src/server.rs` — Added mesh_status: None to 2 OrchestratorStatus construction sites
- `crates/assay-mcp/tests/mcp_handlers.rs` — Added mesh_status: None to 1 OrchestratorStatus construction site

## Forward Intelligence

### What the next slice should know

- The `run_mesh()` threading model (`thread::scope` + `Arc<AtomicUsize> active_count` + routing thread exits when count == 0) is the pattern S03's `run_gossip()` should follow — coordinator thread replaces routing thread, same termination signal
- `persist_state` is now `pub(crate)` in `executor.rs` and is safe to reuse in `gossip.rs` — no need to duplicate
- `OrchestratorStatus` needs `gossip_status: Option<GossipStatus>` added with the same `serde(default, skip_serializing_if)` pattern as `mesh_status` — all existing construction sites will need `gossip_status: None` added (currently ~10 sites across assay-core, assay-mcp)

### What's fragile

- `Suspect` state is unreachable in current implementation — if S04 adds heartbeat polling, it must distinguish sessions that completed before the heartbeat timeout from sessions that crashed silently; the `completed` sentinel file is the authoritative signal
- Routing thread poll interval is hardcoded at 50ms — suitable for tests; for production with many sessions and large files, this may need configuration via `MeshConfig.routing_poll_interval_ms`

### Authoritative diagnostics

- `cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status` — canonical source of truth for member states and messages_routed; readable immediately after `run_mesh()` returns
- `RUST_LOG=assay_core=debug cargo test -- mesh --nocapture` — shows routing events in real time; "routed message" lines confirm file moved

### What assumptions changed

- The plan mentioned creating a separate `session_dirs` map; the implementation uses `name → mesh_dir` where the routing thread looks in `mesh_dir/outbox/<target_name>/` — cleaner than tracking inbox and outbox separately
