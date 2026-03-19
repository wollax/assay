---
id: T03
parent: S02
milestone: M004
provides:
  - Full run_mesh() implementation replacing stub; both integration tests pass
key_files:
  - crates/assay-core/src/orchestrate/mesh.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
key_decisions:
  - Spawn all session workers at once inside thread::scope; bounded concurrency via (Mutex<usize>, Condvar) counting semaphore — workers self-block until a slot opens
  - Routing thread polls every 50ms; exits when active_count AtomicUsize reaches 0 (decremented by each worker after runner returns)
  - Roster layer format: "Outbox: <path>" on its own line so integration test's writer runner can parse it with starts_with("Outbox: ")
  - outcomes vec is built post-scope from final session_statuses — avoids storing pipeline results separately, constructs synthetic PipelineResult for mesh completions
  - assay-mcp server.rs and tests/mcp_handlers.rs had OrchestratorStatus construction sites missing mesh_status: None; added as part of this task
patterns_established:
  - Mesh workers acquire semaphore slot at start, release on completion — same condvar pattern as executor.rs DAG dispatch but without the coordinator loop
  - active_count Arc<AtomicUsize> is the only synchronization between session workers and the routing thread; routing thread runs until count reaches 0
  - Best-effort persist_state inside worker after updating session_statuses_arc and mesh_status_arc (clone both under lock, persist outside lock)
observability_surfaces:
  - "tracing::info!(session, \"mesh session starting\") per session launch"
  - "tracing::debug!(from, to, file, \"routed message\") per routing event"
  - "tracing::warn!(target, source, \"unknown outbox target\") for unrecognized routing targets"
  - "tracing::warn!(session, \"depends_on is ignored in Mesh mode\") for sessions with non-empty depends_on"
  - "state.json at .assay/orchestrator/<run_id>/state.json — mesh_status.members[*].state shows Completed vs Dead; mesh_status.messages_routed shows routing count"
  - "RUST_LOG=assay_core=debug cargo test -- mesh --nocapture to see routing events"
duration: ~45min
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T03: Implement run_mesh() full body

**Replaced the `run_mesh()` stub with a complete peer-mesh executor: inbox/outbox dirs per session, roster PromptLayer injection, background routing thread, bounded-concurrency session workers, MeshMemberState tracking, and state.json persistence — both integration tests pass.**

## What Happened

The stub was replaced wholesale. The implementation:

1. **Pre-scope setup**: Creates `.assay/orchestrator/<run_id>/mesh/<name>/inbox` and `outbox` for every session. Builds a roster `PromptLayer` (kind: System, name: "mesh-roster", priority: -5) injected into each session clone. The roster includes `Outbox: <path>` on a dedicated line (so the writer integration test can parse it) plus `Peer: <name>  Inbox: <path>` for each other session.

2. **Shared state**: `Arc<AtomicUsize> active_count` for routing thread termination; `Arc<Mutex<MeshStatus>>` and `Arc<Mutex<Vec<SessionStatus>>>` for cross-worker state; `Arc<(Mutex<usize>, Condvar)>` semaphore for bounded concurrency.

3. **thread::scope**:
   - **Routing thread**: Polls each session's `outbox/<target_name>/` directory every 50ms, renames files to `<target_name>/inbox/<filename>`, increments `mesh_status.messages_routed`. Exits when `active_count == 0`.
   - **Session workers**: Each acquires a semaphore slot (blocks if `in_flight >= effective_concurrency`), marks session Running, calls `session_runner` inside `panic::catch_unwind(AssertUnwindSafe(...))`, writes `completed` sentinel, updates `MeshMemberState` (Completed on Ok, Dead on Err/panic), best-effort persists snapshot, decrements `active_count`, releases semaphore slot.

4. **Post-scope**: Builds final `OrchestratorStatus` with `phase = Completed | PartialFailure`, persists to `state.json`, builds `outcomes` vec from final statuses.

Also fixed two `OrchestratorStatus` construction sites in `assay-mcp/src/server.rs` and one in `assay-mcp/tests/mcp_handlers.rs` that were missing `mesh_status: None` — these caused clippy/test compilation failures.

## Verification

```
cargo test -p assay-core --features orchestrate -- mesh --nocapture
```
- `test_mesh_mode_message_routing` ✓ — routing thread moved `msg.txt` from writer's outbox to reader's inbox; `messages_routed >= 1`
- `test_mesh_mode_completed_not_dead` ✓ — both alpha/beta sessions show `MeshMemberState::Completed`
- 4 new unit tests in mesh.rs ✓

```
cargo test -p assay-core --features orchestrate
```
All 766 filtered + integration tests pass.

```
cargo test -p assay-types --features orchestrate
```
All 61 tests pass (schema snapshots unchanged).

```
just ready
```
fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓

## Diagnostics

- `cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status` — shows `members` list with per-session `state` (alive/completed/dead) and `last_heartbeat_at`, plus `messages_routed` counter
- `RUST_LOG=assay_core=debug cargo test -- mesh --nocapture` — shows per-message routing events (from, to, filename) and session launch info
- `MeshMemberState::Dead` in state.json indicates crash or pipeline error; `Completed` indicates normal exit

## Deviations

- The task plan mentioned `session_dirs: Vec<(String, PathBuf)>` pointing to mesh dir. The routing thread correctly looks in `session_dir.join("outbox")` rather than a separate outbox field — consistent with the spec.
- The unit tests from the stub (`run_mesh_returns_empty_result`, `run_mesh_emits_warn_for_depends_on`) were replaced with implementation-accurate tests as planned in step 8. The old `run_mesh_emits_warn_for_depends_on` became a new test that verifies `depends_on` sessions still run successfully.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/mesh.rs` — full implementation replacing stub; 4 unit tests covering runner calls, roster injection, state persistence, and depends_on warning
- `crates/assay-mcp/src/server.rs` — added `mesh_status: None` to two `OrchestratorStatus` construction sites in test helpers
- `crates/assay-mcp/tests/mcp_handlers.rs` — added `mesh_status: None` to one `OrchestratorStatus` construction site
