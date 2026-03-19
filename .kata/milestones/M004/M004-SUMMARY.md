---
id: M004
provides:
  - OrchestratorMode enum (Dag | Mesh | Gossip) with snake_case serde and locked schema snapshot
  - MeshConfig and GossipConfig structs on RunManifest (optional, backward-compatible) with locked schema snapshots
  - run_mesh() full implementation: parallel dispatch, roster PromptLayer injection, inbox/outbox dirs, routing thread, SWIM membership, state persistence
  - MeshMemberState / MeshMemberStatus / MeshStatus types with locked schema snapshots
  - run_gossip() full implementation: parallel dispatch, coordinator mpsc thread, atomic knowledge.json writes, PromptLayer injection, gossip_status persistence
  - KnowledgeEntry / KnowledgeManifest / GossipStatus types with locked schema snapshots
  - OrchestratorStatus extended with mesh_status and gossip_status optional fields (backward-compatible)
  - execute_mesh() / execute_gossip() wired to real HarnessWriter session runners in CLI
  - MCP orchestrate_status surfaces mesh_status / gossip_status from state.json
  - integration_modes.rs: all-modes regression suite (DAG + Mesh + Gossip)
  - mesh_integration.rs and gossip_integration.rs: filesystem-level integration tests
  - Race condition fix: #[serial] on two server.rs unit tests using set_current_dir
key_decisions:
  - D052: Mode dispatch via free functions (run_mesh, run_gossip) — zero-trait convention
  - D053: Mesh/Gossip modes ignore depends_on with tracing::warn
  - D054: OrchestratorStatus extended with optional mode-specific fields via serde(default, skip_serializing_if)
  - D055: MeshConfig/GossipConfig as optional top-level RunManifest fields — flat manifest, no polymorphic union
  - D056: impl Default for RunManifest to unblock test struct literals across workspace
  - D057: persist_state made pub(crate) in executor.rs for reuse by mesh.rs
  - D058: Mesh roster PromptLayer uses "Outbox: <path>" as machine-parseable line
  - D059: Gossip PromptLayer uses "Knowledge manifest: <path>" as machine-parseable line
  - D060: Coordinator thread uses mpsc channel with drain loop for completion events
  - D061: execute_mesh/execute_gossip use HarnessWriter pattern without merge phase
patterns_established:
  - Mode dispatch in CLI: match on manifest.mode before needs_orchestration(); Dag falls through, Mesh/Gossip return early
  - MCP mode routing: match arm before DAG spawn_blocking block; each mode calls executor via spawn_blocking and returns early
  - thread::scope with routing/coordinator thread + N worker threads sharing Arc<AtomicUsize> active_count as termination signal
  - Bounded concurrency via (Mutex<usize>, Condvar) counting semaphore — same pattern as executor.rs DAG dispatch
  - persist_state reuse: pub(crate) atomic write helper shared across executor, mesh, gossip
  - mpsc drain loop (while let Ok(c) = rx.try_recv()) after Disconnected prevents last-message loss
  - MCP status round-trip tests: write realistic OrchestratorStatus to state.json, call orchestrate_status(), assert JSON fields
  - Unit tests using set_current_dir must be marked #[serial] to avoid racing in the same binary
observability_surfaces:
  - "cat .assay/orchestrator/<run_id>/state.json | jq .mesh_status — members list with state + messages_routed"
  - "cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status — sessions_synthesized + coordinator_rounds + manifest path"
  - "cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '.entries | length' — synthesis count"
  - "RUST_LOG=assay_core=debug — per-message routing events (mesh) and coordinator rounds (gossip)"
  - stderr: "mode: mesh — N session(s)" / "mode: gossip — N session(s)" on CLI entry
  - orchestrate_status MCP response: ["status"]["mesh_status"] / ["status"]["gossip_status"] non-null after mesh/gossip run
requirement_outcomes:
  - id: R034
    from_status: active
    to_status: validated
    proof: "S01 — OrchestratorMode enum with schema snapshot locked; mode field on RunManifest with serde(default) backward-compatible; CLI and MCP dispatch routing exercised by unit tests (3 CLI + 2 MCP); all 1222+ tests pass; just ready green. S04 completes wiring with real HarnessWriter runners."
  - id: R035
    from_status: active
    to_status: validated
    proof: "S02 — test_mesh_mode_completed_not_dead proves parallel launch with roster PromptLayer injection; all sessions start without DAG ordering; depends_on emits warn and is ignored; state.json persists correct membership states; schema snapshots locked. S04 — integration_modes.rs test_all_modes_mesh_executes_two_sessions confirms Completed outcomes."
  - id: R036
    from_status: active
    to_status: validated
    proof: "S02 — test_mesh_mode_message_routing proves outbox→inbox routing with real filesystem ops; messages_routed counter accurate; MeshMemberState Completed vs Dead distinguishes normal exit from crash; routing thread polls every 50ms, exits when active_count==0. S04 — orchestrate_status_returns_mesh_status MCP test asserts messages_routed=3 surfaces correctly."
  - id: R037
    from_status: active
    to_status: validated
    proof: "S03 — test_gossip_mode_knowledge_manifest proves 3 mock sessions → knowledge.json with 3 entries, gossip_status.sessions_synthesized == 3; coordinator thread uses mpsc with drain loop; GossipStatus/KnowledgeEntry/KnowledgeManifest schema snapshots locked; just ready green. S04 — integration_modes.rs test_all_modes_gossip_executes_two_sessions confirms outcomes."
  - id: R038
    from_status: active
    to_status: validated
    proof: "S03 — test_gossip_mode_manifest_path_in_prompt_layer proves each session receives a 'gossip-knowledge-manifest' PromptLayer with 'Knowledge manifest: <path>' line under the run's orchestrator directory; atomic knowledge.json writes via tempfile+rename+sync_all. S04 — knowledge.json entry count verified in integration_modes test."
duration: ~5h total (S01: ~2.5h, S02: ~80min, S03: ~1.5h, S04: ~45min)
verification_result: passed
completed_at: 2026-03-18
---

# M004: Coordination Modes — Mesh and Gossip

**Two new coordination modes (Mesh and Gossip) ship alongside the existing DAG executor: Mesh enables file-based peer messaging between parallel sessions with SWIM-inspired membership tracking; Gossip enables emergent cross-pollination via a coordinator-assembled knowledge manifest — all mode-dispatched from a single `mode` field on `RunManifest`, with 1271 tests passing and 0 warnings.**

## What Happened

M004 was executed across four slices with a clear build-up pattern: types → integration contracts → full implementation → CLI/MCP wiring.

**S01 (Mode Infrastructure)** established the type contract and dispatch scaffolding. `OrchestratorMode` (Dag | Mesh | Gossip), `MeshConfig`, and `GossipConfig` were added to `assay-types::orchestrate` with full derives, schema snapshots, and `inventory::submit!` registration. `RunManifest` gained three backward-compatible fields (`mode`, `mesh_config`, `gossip_config`) without breaking the existing schema snapshot — the diff was purely additive. Stubs for `run_mesh()` and `run_gossip()` were wired through CLI `execute()` and MCP `orchestrate_run`. A cascade of struct-literal test failures across the workspace was resolved by adding `impl Default for RunManifest` (D056) — avoiding hundreds of individual test literal edits without changing the serde schema.

**S02 (Mesh Mode)** replaced the `run_mesh()` stub with a complete implementation. Per the proof-first pattern, integration tests were written first as failing contracts: `test_mesh_mode_message_routing` (writer session writes outbox file targeting reader, assert file arrives in inbox with `messages_routed >= 1`) and `test_mesh_mode_completed_not_dead` (sessions complete normally → all show `Completed` state). The implementation used `thread::scope` with a routing thread (polls outbox subdirs every 50ms, renames files to target inboxes, exits when `active_count == 0`) alongside N worker threads with bounded concurrency via a `(Mutex<usize>, Condvar)` semaphore. Roster `PromptLayer` injection (D058: "Outbox: <path>" and "Peer: <name> Inbox: <path>" lines) gives each session machine-parseable peer discovery. `persist_state` was elevated to `pub(crate)` (D057) for reuse by mesh.rs and gossip.rs.

**S03 (Gossip Mode)** followed the mesh blueprint. `KnowledgeEntry`, `KnowledgeManifest`, and `GossipStatus` were added to `assay-types` with locked schema snapshots. A `GossipCompletion` struct carries per-session results from workers to a coordinator thread via `mpsc::channel`. The coordinator writes atomic `knowledge.json` updates (same tempfile+rename+sync_all pattern as `persist_state`). Sessions receive a `"gossip-knowledge-manifest"` PromptLayer with a `"Knowledge manifest: <path>"` line (D059) injected at launch — running sessions can read the file at any point during execution. A critical correctness detail: a `while let Ok(c) = rx.try_recv()` drain loop after `Disconnected` prevents completions from being lost when all workers finish rapidly (D060).

**S04 (Integration + Observability)** wired the final connections. `execute_mesh()` and `execute_gossip()` in `run.rs` were rewritten from `unreachable!()` stubs to real `Box<HarnessWriter>` session runners matching the DAG path — both print mode-specific startup lines and iterate outcomes identically (D061: no merge phase for Mesh/Gossip). Two MCP handler tests assert that realistic `mesh_status` / `gossip_status` values round-trip correctly through `orchestrate_status`. A new `integration_modes.rs` suite exercises all three mode dispatchers in a single file. A pre-existing race condition in two `server.rs` unit tests using `set_current_dir` without `#[serial]` was found and fixed.

## Cross-Slice Verification

**Success criterion 1** — `assay run manifest.toml` with `mode = "mesh"` launches sessions in parallel, each with a roster prompt layer, orchestrator routes outbox messages to target inboxes, and membership states are tracked in `orchestrate_status`:
- `test_mesh_mode_message_routing` (mesh_integration.rs): real filesystem operations prove outbox→inbox routing with `messages_routed >= 1`
- `test_mesh_mode_completed_not_dead` (mesh_integration.rs): all members show `Completed` state (not `Dead`) after normal exit
- `test_all_modes_mesh_executes_two_sessions` (integration_modes.rs): mock runner confirms 2 Completed outcomes
- `orchestrate_status_returns_mesh_status` (mcp_handlers.rs): `mesh_status` with `messages_routed` surfaces in MCP response ✓

**Success criterion 2** — `assay run manifest.toml` with `mode = "gossip"` launches sessions with knowledge manifest path in prompt layers, coordinator updates manifest atomically, `orchestrate_status` returns `gossip_status`:
- `test_gossip_mode_knowledge_manifest` (gossip_integration.rs): 3 mock sessions → `knowledge.json` with 3 entries, `sessions_synthesized == 3`
- `test_gossip_mode_manifest_path_in_prompt_layer` (gossip_integration.rs): each session receives `"gossip-knowledge-manifest"` PromptLayer with correct path
- `test_all_modes_gossip_executes_two_sessions` (integration_modes.rs): knowledge.json entry count verified
- `orchestrate_status_returns_gossip_status` (mcp_handlers.rs): `gossip_status` surfaces in MCP response ✓

**Success criterion 3** — Existing manifest with no `mode` field runs with identical DAG behavior, all 1222+ tests continue to pass:
- `cargo test --workspace --features orchestrate` — **1271 passed, 0 failed** (exceeds 1222+ threshold)
- `test_all_modes_dag_executes_two_sessions` (integration_modes.rs): DAG path unchanged ✓

**Success criterion 4** — `just ready` passes with fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓:
- Verified after S04 completion: all checks exit 0, 0 compiler warnings ✓

**Success criterion 5** — Schema snapshots locked for all new types:
- 10 new/updated snapshot files in `crates/assay-types/tests/snapshots/`:
  `orchestrator-mode-schema`, `mesh-config-schema`, `gossip-config-schema`, `mesh-member-state-schema`, `mesh-member-status-schema`, `mesh-status-schema`, `gossip-status-schema`, `knowledge-entry-schema`, `knowledge-manifest-schema`, `run-manifest-schema` ✓

## Requirement Changes

- R034: active → validated — OrchestratorMode enum + schema snapshot locked; mode field on RunManifest backward-compatible; CLI and MCP dispatch exercised by tests; all 1271 tests pass
- R035: active → validated — test_mesh_mode_completed_not_dead + integration_modes mesh test prove parallel launch with roster injection and Completed outcomes
- R036: active → validated — test_mesh_mode_message_routing with real filesystem ops proves routing; messages_routed surfaces in MCP orchestrate_status response
- R037: active → validated — test_gossip_mode_knowledge_manifest proves 3 sessions → 3 entries in knowledge.json, sessions_synthesized == 3; integration_modes gossip test confirms outcomes
- R038: active → validated — test_gossip_mode_manifest_path_in_prompt_layer proves gossip-knowledge-manifest PromptLayer injected; atomic knowledge.json writes verified

## Forward Intelligence

### What the next milestone should know

- M004 is fully green — 1271 tests, 0 warnings. All four slices complete. Schema snapshots locked. Stable baseline for M005 planning.
- `orchestrate_run` in `server.rs` has `unreachable!()` stubs for mesh/gossip MCP orchestrate_run paths — intentional (D061 domain: additive post-M004). These are documented known limitations; do not treat as bugs.
- The `thread::scope` + `Arc<AtomicUsize> active_count` termination pattern is now established for both Mesh and Gossip — any future coordination mode should follow this blueprint.
- `impl Default for RunManifest` is in `manifest.rs` (not derived). Any new `deny_unknown_fields` field on `RunManifest` needs a matching entry in that `Default` impl — otherwise `..Default::default()` struct literals will fail to compile.
- `persist_state` is `pub(crate)` in `executor.rs` — available to any future mode executor in the `assay-core::orchestrate` module.
- The two `#[serial]` tests in `server.rs` are sensitive to any new test using `set_current_dir` without `#[serial]` in the same binary.

### What's fragile

- `Suspect` state in `MeshMemberState` is defined but unreachable — sessions start `Alive` and transition directly to `Completed` or `Dead`. Heartbeat-based `Alive → Suspect → Dead` transitions require a polling loop that reads heartbeat file timestamps; this was explicitly deferred from S02.
- `last_heartbeat_at` is always `None` in current state.json — field schema is locked and backward-compatible, but the value is never populated until heartbeat writes are implemented.
- Coordinator drain loop (`while let Ok(c) = rx.try_recv()` after `Disconnected`) is critical — removing it causes `sessions_synthesized < actual` in fast-running scenarios.
- `drop(tx)` in `run_gossip()` must remain in the parent thread scope before `thread::scope` waits on workers; moving it outside breaks coordinator termination semantics.

### Authoritative diagnostics

- `cargo test -p assay-types --features orchestrate` — most reliable first check; catches schema drift before it cascades to integration tests
- `cargo test -p assay-core --features orchestrate --test mesh_integration` — isolated mesh routing verification
- `cargo test -p assay-core --features orchestrate --test gossip_integration` — isolated gossip manifest verification
- `cargo test -p assay-core --features orchestrate --test integration_modes -- --nocapture` — verbose all-modes sweep; first signal for cross-mode regression
- `just ready` — authoritative green/red signal for the full workspace

### What assumptions changed

- Adding fields to `RunManifest` (a `deny_unknown_fields` struct) cascades to all struct-literal test constructions across the workspace — not just the immediate file. `impl Default` is the correct mitigation pattern; individual struct-literal updates are fragile and don't scale.
- T03 in S04 was planned as trivial (~15min) but surfaced a pre-existing race condition in server.rs tests that required investigation and `#[serial]` fixes — any future work that adds `set_current_dir` to tests must include `#[serial]`.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — Added OrchestratorMode, MeshConfig, GossipConfig, MeshMemberState, MeshMemberStatus, MeshStatus, KnowledgeEntry, KnowledgeManifest, GossipStatus; extended OrchestratorStatus with mesh_status and gossip_status optional fields
- `crates/assay-types/src/lib.rs` — Re-exported all new orchestrate types under #[cfg(feature = "orchestrate")]
- `crates/assay-types/src/manifest.rs` — Added mode, mesh_config, gossip_config fields; impl Default for RunManifest; backward-compatibility unit tests
- `crates/assay-types/tests/schema_snapshots.rs` — Added 9 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-mode-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-config-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-config-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-state-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-member-status-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__mesh-status-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__gossip-status-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-entry-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__knowledge-manifest-schema.snap` — New, locked
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — Updated with mode, mesh_config, gossip_config additive fields
- `crates/assay-types/tests/snapshots/schema_snapshots__orchestrator-status-schema.snap` — Updated with mesh_status and gossip_status additive fields
- `crates/assay-types/tests/schema_roundtrip.rs` — Fixed RunManifest literal
- `crates/assay-core/src/orchestrate/mod.rs` — Added pub mod mesh; pub mod gossip
- `crates/assay-core/src/orchestrate/executor.rs` — persist_state now pub(crate); OrchestratorStatus construction sites updated with mesh_status/gossip_status: None
- `crates/assay-core/src/orchestrate/mesh.rs` — Full implementation: inbox/outbox dirs, roster PromptLayer, thread::scope routing thread, SWIM membership, state persistence; 4 unit tests
- `crates/assay-core/src/orchestrate/gossip.rs` — Full implementation: GossipCompletion, persist_knowledge_manifest(), coordinator mpsc thread, PromptLayer injection, gossip_status persistence; unit test
- `crates/assay-core/src/orchestrate/dag.rs` — Fixed test make_manifest helper (..Default::default())
- `crates/assay-core/src/manifest.rs` — Fixed test struct literals
- `crates/assay-core/src/pipeline.rs` — Fixed test RunManifest literal
- `crates/assay-core/tests/orchestrate_integration.rs` — Fixed make_manifest helper
- `crates/assay-core/tests/mesh_integration.rs` — New: test_mesh_mode_message_routing, test_mesh_mode_completed_not_dead
- `crates/assay-core/tests/gossip_integration.rs` — New: test_gossip_mode_knowledge_manifest, test_gossip_mode_manifest_path_in_prompt_layer
- `crates/assay-core/tests/integration_modes.rs` — New: all-modes regression suite (DAG + Mesh + Gossip)
- `crates/assay-cli/src/commands/run.rs` — Mode dispatch in execute(), execute_mesh/execute_gossip with real HarnessWriter runners; stderr mode display; 3 unit tests
- `crates/assay-mcp/src/server.rs` — OrchestratorMode import, mode guard conditioned on Dag, Mesh/Gossip routing arms; mesh_status/gossip_status: None at all OrchestratorStatus construction sites; #[serial] on 2 set_current_dir tests
- `crates/assay-mcp/tests/mcp_handlers.rs` — mesh_status/gossip_status: None at construction sites; orchestrate_status_returns_mesh_status and orchestrate_status_returns_gossip_status tests
