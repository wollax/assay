# S02: Mesh Mode

**Goal:** Replace the `run_mesh()` stub with a complete implementation that launches all sessions in parallel, injects a roster `PromptLayer` into each session, creates `inbox/` and `outbox/` directories, runs a background routing thread that polls outboxes and moves files to target inboxes, and tracks SWIM-inspired membership state via heartbeat/completed sentinels — proven by an integration test using mock session runners.

**Demo:** `cargo test -p assay-core --features orchestrate -- mesh` shows two integration tests pass: one proving a message file written to session "writer"'s outbox arrives in session "reader"'s inbox with `mesh_status.messages_routed == 1`, and one proving completed sessions are classified as `Completed` (not `Dead`) in membership state.

## Must-Haves

- `MeshMemberState` enum (`Alive` | `Suspect` | `Dead` | `Completed`) with schema snapshot locked
- `MeshMemberStatus` struct (`name`, `state`, `last_heartbeat_at`) with schema snapshot locked
- `MeshStatus` struct (`members`, `messages_routed`) with schema snapshot locked
- `OrchestratorStatus` extended with `mesh_status: Option<MeshStatus>` — updated schema snapshot locked; backward-compatible with existing `state.json` files (field defaults to `None`)
- `persist_state` made `pub(crate)` in `executor.rs` so `mesh.rs` can reuse it without duplication
- Integration test `test_mesh_mode_message_routing`: 2 mock sessions — "writer" writes an outbox file targeting "reader", routing thread routes it; asserts file exists in reader's inbox AND `state.json` `mesh_status.messages_routed == 1`
- Integration test `test_mesh_mode_completed_not_dead`: 2 mock sessions both complete normally; asserts all `MeshMemberStatus` entries have state `Completed` (not `Dead`) in final `state.json`
- `run_mesh()` full implementation: parallel dispatch (no DAG, all sessions launch immediately), roster `PromptLayer` injected into each session clone before runner call, `inbox/` and `outbox/` dirs created under `.assay/orchestrator/<run_id>/mesh/<name>/`, routing thread runs inside `thread::scope` with `Arc<AtomicUsize>` active-sessions counter as termination signal, session workers write `completed` sentinel after runner returns, `OrchestratorStatus` with `mesh_status` persisted to `state.json` after each session completes
- `max_concurrency` from `OrchestratorConfig` respected (bounded parallel dispatch)
- `just ready` passes with 0 warnings; all 1222+ existing tests continue to pass

## Proof Level

- This slice proves: **integration** — real filesystem operations (directory creation, file moves, JSON persistence) with mock session runners; no live agent processes
- Real runtime required: no (mock runners only for automated tests; real Claude agents are UAT-only, same pattern as M003)
- Human/UAT required: no (automated integration tests prove the routing and membership mechanics)

## Verification

- `cargo test -p assay-types --features orchestrate -- schema_snapshots` — all 4 schema snapshots pass (MeshMemberState, MeshMemberStatus, MeshStatus, OrchestratorStatus updated)
- `cargo test -p assay-core --features orchestrate -- mesh` — both integration tests pass: `test_mesh_mode_message_routing` and `test_mesh_mode_completed_not_dead`
- `cargo test -p assay-core --features orchestrate` — all existing executor/DAG/integration tests continue to pass (regression)
- `just ready` — fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓

## Observability / Diagnostics

- Runtime signals: `tracing::info!` at session launch (name, inbox path), `tracing::debug!` per routed message (from_session → to_session, filename), `tracing::warn!` per unrecognized outbox target name
- Inspection surfaces: `state.json` under `.assay/orchestrator/<run_id>/` — contains `mesh_status` with `members` list (per-session state + last_heartbeat_at) and `messages_routed` counter; readable by `orchestrate_status` MCP tool (S04 wires the response field)
- Failure visibility: session state transitions visible in `state.json` per-member (Alive → Suspect → Dead or → Completed); `messages_routed` counter reveals whether routing thread processed any messages; unrecognized target names warn to stderr via tracing
- Redaction constraints: none (no secrets in mesh coordination data)

## Integration Closure

- Upstream surfaces consumed: `OrchestratorMode::Mesh` dispatch routing from S01; `MeshConfig` type for timing configuration; `run_mesh()` stub signature (replaced, signature unchanged per D052)
- New wiring introduced in this slice: `run_mesh()` full implementation writes `state.json` with `mesh_status` field; integration tests in `crates/assay-core/tests/mesh_integration.rs` exercise the filesystem-level routing contract
- What remains before the milestone is truly usable end-to-end: S03 (Gossip mode), S04 (`orchestrate_status` MCP response surfaces `mesh_status` field, CLI shows mode in output)

## Tasks

- [x] **T01: Add mesh status types and extend OrchestratorStatus** `est:45m`
  - Why: Establishes the type contract for mesh membership before implementation — locking schema snapshots early prevents drift and makes S04's MCP surfacing additive-only
  - Files: `crates/assay-types/src/orchestrate.rs`, `crates/assay-types/src/lib.rs`, `crates/assay-types/tests/schema_snapshots.rs`, `crates/assay-core/src/orchestrate/executor.rs`
  - Do: Add `MeshMemberState` enum (Alive/Suspect/Dead/Completed, full derives, snake_case serde, no deny_unknown_fields on enum) and `MeshMemberStatus`/`MeshStatus` structs (deny_unknown_fields, full derives, DateTime<Utc> for last_heartbeat_at) to `orchestrate.rs`; add `inventory::submit!` entries for all three; add `mesh_status: Option<MeshStatus>` with `serde(default, skip_serializing_if = "Option::is_none")` to `OrchestratorStatus`; re-export all three from `lib.rs` under `#[cfg(feature = "orchestrate")]`; add snapshot tests for all three new types in `schema_snapshots.rs`; make `persist_state` `pub(crate)` in `executor.rs`; run `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` to regenerate snapshots for new types and updated `OrchestratorStatus`; run `cargo test -p assay-types --features orchestrate` to confirm all pass
  - Verify: `cargo test -p assay-types --features orchestrate` passes; snapshot files exist for MeshMemberState, MeshMemberStatus, MeshStatus; OrchestratorStatus snapshot is updated; `cargo test -p assay-core --features orchestrate` still passes (OrchestratorStatus deserialization backward-compatible)
  - Done when: All assay-types tests pass, 3 new snapshot files exist, existing OrchestratorStatus snapshot is regenerated and accepted, executor.rs `persist_state` is `pub(crate)`

- [x] **T02: Write failing integration test for mesh mode message routing** `est:30m`
  - Why: Defines the observable contract before implementation — test failure confirms the stub is the obstacle, not a test bug; forces precise API agreement between test setup and implementation
  - Files: `crates/assay-core/tests/mesh_integration.rs`
  - Do: Create `crates/assay-core/tests/mesh_integration.rs`; add `test_mesh_mode_message_routing` — 2-session mesh manifest ("writer", "reader"), mock runner for "writer" creates `outbox/reader/msg.txt` in its mesh dir then sleeps 200ms, mock runner for "reader" sleeps 300ms; after `run_mesh()` returns assert `mesh/reader/inbox/msg.txt` exists and `state.json` `mesh_status.messages_routed == 1`; add `test_mesh_mode_completed_not_dead` — 2 sessions both succeed, assert all `MeshMemberStatus` entries in final state.json have `state: "completed"`; compile and confirm tests fail (stub doesn't route messages)
  - Verify: `cargo test -p assay-core --features orchestrate -- mesh` compiles and both tests fail predictably (not a compile error — a test assertion failure or wrong counts)
  - Done when: Both tests compile and fail with clear assertion messages; no compile errors

- [x] **T03: Implement run_mesh() full body** `est:1h30m`
  - Why: The core deliverable — replaces the stub with real parallel dispatch, roster injection, inbox/outbox directory creation, routing thread, heartbeat/completed sentinel tracking, and state persistence
  - Files: `crates/assay-core/src/orchestrate/mesh.rs`
  - Do: Replace stub body with full implementation — (1) build `run_dir` and `run_id`; (2) for each session compute effective name (`session.name.as_deref().unwrap_or(&session.spec)`), create `.assay/orchestrator/<run_id>/mesh/<name>/inbox/` and `.../outbox/` dirs; (3) build `name → inbox_path` map for routing thread; (4) build roster `PromptLayer` for each session listing all peers and their inbox paths (kind=System, priority=-5, name="mesh-roster"); (5) clone sessions and append roster layer to each clone's `prompt_layers` (do not mutate manifest); (6) initialize `MeshStatus` with all members as `Alive`; construct `Arc<AtomicUsize> active` = sessions.len(), `Arc<Mutex<MeshStatus>> mesh_status`, `Arc<Mutex<Vec<SessionStatus>>> session_statuses`; (7) enter `std::thread::scope`: spawn routing thread (polls outbox subdirs every 50ms, moves files to target inbox dirs, increments messages_routed, emits tracing::debug, loops until active==0); spawn one worker per session up to max_concurrency (use same bounded-concurrency pattern via semaphore-style AtomicUsize or sequential dispatch for simplicity); each worker calls `panic::catch_unwind(AssertUnwindSafe(|| session_runner(&session_clone, pipeline_config)))`, writes `completed` sentinel file, decrements active, updates session status and member state to Completed, calls `persist_state(run_dir, &status_with_mesh)` best-effort; (8) after scope exits build `OrchestratorResult` and persist final status; return Ok(result)
  - Verify: `cargo test -p assay-core --features orchestrate -- mesh` — both integration tests pass; `cargo test -p assay-core --features orchestrate` — all executor/integration tests still pass
  - Done when: `test_mesh_mode_message_routing` and `test_mesh_mode_completed_not_dead` both pass; no existing tests broken

- [x] **T04: just ready — lint, test, snapshot lockdown** `est:20m`
  - Why: Confirms zero warnings (lint discipline), all 1222+ tests pass (regression), and schema snapshots are stable and committed
  - Files: any files with lint warnings (minor fixes only)
  - Do: Run `just ready`; fix any clippy warnings (likely `allow(dead_code)` removals or unused import cleanup in mesh.rs); confirm no snapshot drift; if snapshot drift, re-run `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate` and accept; verify git status shows all .snap files committed
  - Verify: `just ready` exits 0 with output `fmt ✓ lint ✓ test ✓ deny ✓`; `git diff --name-only crates/assay-types/tests/snapshots/` shows no unstaged changes
  - Done when: `just ready` green with 0 warnings; all snapshot files committed and stable

## Files Likely Touched

- `crates/assay-types/src/orchestrate.rs` — Add MeshMemberState, MeshMemberStatus, MeshStatus, extend OrchestratorStatus, add inventory entries
- `crates/assay-types/src/lib.rs` — Re-export new mesh types under `#[cfg(feature = "orchestrate")]`
- `crates/assay-types/tests/schema_snapshots.rs` — Add snapshot tests for 3 new types
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-state-schema.snap` — New
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-status-schema.snap` — New
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-status-schema.snap` — New
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — Updated (new optional field)
- `crates/assay-core/src/orchestrate/executor.rs` — Make `persist_state` `pub(crate)`
- `crates/assay-core/src/orchestrate/mesh.rs` — Full implementation replaces stub
- `crates/assay-core/tests/mesh_integration.rs` — New integration tests
