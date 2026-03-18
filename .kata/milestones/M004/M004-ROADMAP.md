# M004: Coordination Modes — Mesh and Gossip

**Vision:** Assay gains two new coordination modes alongside its existing DAG executor. Mesh mode launches sessions in parallel, injects each with a peer roster, and routes file-based messages between agents. Gossip mode launches sessions in parallel, runs a coordinator thread that synthesizes completed sessions' gate results into a knowledge manifest, and injects the manifest path into each session's prompt layer so agents can read what peers have accomplished. Both modes are selected via a `mode` field on `RunManifest`; all existing DAG behavior is preserved.

## Success Criteria

- `assay run manifest.toml` where `manifest.toml` has `mode = "mesh"` launches sessions in parallel, each with a roster prompt layer naming peers and their inbox paths; the orchestrator routes outbox messages to target inboxes; membership states (alive/suspect/dead) are tracked and visible in `orchestrate_status`
- `assay run manifest.toml` where `manifest.toml` has `mode = "gossip"` launches sessions in parallel with a knowledge manifest path in their prompt layer; the coordinator updates the manifest atomically as sessions complete; `orchestrate_status` returns `gossip_status` with sessions_synthesized count and manifest path
- An existing manifest with no `mode` field (or `mode = "dag"`) runs with identical behavior to M003 — all 1222+ tests continue to pass
- `just ready` passes: fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓
- Schema snapshots locked for `OrchestratorMode`, `MeshConfig`, `GossipConfig`, `MeshStatus`, `GossipStatus`, `KnowledgeManifest`, and updated `RunManifest`

## Key Risks / Unknowns

- **Concurrent routing thread in thread::scope** — Mesh needs a background message-routing thread running alongside session worker threads within `std::thread::scope`. Composing a routing loop with the existing condvar dispatch loop requires careful synchronization design. This is the highest risk item.
- **Heartbeat vs agent lifecycle** — Agents run until completion and exit; they are not persistent servers. "Heartbeat" files must be interpreted relative to session run state — a session that completed is not "dead", it's done. Membership tracking must distinguish completion from crash/silence.
- **Schema backward compatibility** — `RunManifest` has a locked schema snapshot. Adding `mode` with `serde(default)` must not break the existing snapshot test; the snapshot must be regenerated and re-locked.

## Proof Strategy

- Concurrent routing thread → retire in S02 by building `run_mesh()` with a routing thread that routes at least one message in an integration test with mock runners writing outbox files
- Heartbeat vs agent lifecycle → retire in S02 by testing that a completed session is classified as `Completed` (not `Dead`) in mesh membership state
- Schema backward compatibility → retire in S01 by adding `mode` field, running `cargo test --package assay-types` to see snapshot test fail, regenerating the snapshot, and verifying it's accepted

## Verification Classes

- Contract verification: unit tests for mode dispatch routing, roster prompt layer construction, message routing logic, coordinator knowledge manifest assembly; schema snapshot tests for all new types
- Integration verification: integration tests using mock session runners (per D032) that prove mesh outbox→inbox routing and gossip knowledge manifest population with real filesystem operations
- Operational verification: DAG regression — all existing 1222+ tests still pass; `just ready` green
- UAT / human verification: real `claude -p` agents using the roster/gossip manifest path to actually coordinate — manual UAT only (same pattern as M003's real Claude conflict resolution UAT)

## Milestone Definition of Done

This milestone is complete only when all are true:

- S01 (mode infrastructure) is complete: `OrchestratorMode` enum, `mode` field on `RunManifest`, dispatch routing, all schema snapshots updated and locked
- S02 (Mesh mode) is complete: `run_mesh()` with parallel dispatch, roster injection, inbox/outbox directories, message routing thread, SWIM membership, integration test with mock runners exercising message routing
- S03 (Gossip mode) is complete: `run_gossip()` with parallel dispatch, coordinator thread, knowledge manifest assembly, manifest path injection at launch, integration test with mock runners verifying manifest population
- S04 (Integration + observability) is complete: `orchestrate_status` returns mode-specific fields, CLI surfaces mode in output, all three modes have end-to-end integration test coverage, `just ready` green with 0 warnings
- All schema snapshots for new types are locked (not just generated — they must be committed and stable)
- No existing MCP tool signatures changed (additive only, per D005)

## Requirement Coverage

- Covers: R034, R035, R036, R037, R038
- Partially covers: R027 (OTel instrumentation) — M004 establishes mesh/gossip observability surfaces but OTel wiring remains deferred
- Leaves for later: R027 (OTel wiring itself)
- Orphan risks: none

## Slices

- [ ] **S01: Mode infrastructure** `risk:low` `depends:[]`
  > After this: `mode = "mesh"` and `mode = "gossip"` parse in RunManifest TOML, dispatch to correct executor entry point (stub implementations), existing DAG tests pass, schema snapshots updated.

- [ ] **S02: Mesh mode** `risk:high` `depends:[S01]`
  > After this: Integration test with mock runners proves parallel launch with roster prompt layers, outbox message files are routed to target inboxes, and membership states are tracked in state.json; `just ready` passes.

- [ ] **S03: Gossip mode** `risk:high` `depends:[S01]`
  > After this: Integration test with mock runners proves parallel launch with knowledge manifest path in prompt layers, coordinator updates knowledge.json as sessions complete, and `gossip_status` is visible in `orchestrate_status`; `just ready` passes.

- [ ] **S04: Integration + observability** `risk:low` `depends:[S02,S03]`
  > After this: All three modes have end-to-end integration coverage, `orchestrate_status` returns mode-specific state (mesh_status or gossip_status), CLI shows mode in run output, `just ready` green with 0 warnings.

## Boundary Map

### S01 → S02

Produces:
- `OrchestratorMode` enum in `assay-types` (`dag`, `mesh`, `gossip`) with schema snapshot
- `mode: OrchestratorMode` field on `RunManifest` with `serde(default = "dag")`; updated schema snapshot
- `MeshConfig` type in `assay-types` (`heartbeat_interval_secs`, `suspect_timeout_secs`, `dead_timeout_secs`) with schema snapshot
- `mesh_config: Option<MeshConfig>` field on `RunManifest`
- Dispatch routing in orchestration entry point: `match mode { Dag => run_orchestrated(), Mesh => run_mesh(), Gossip => run_gossip() }`
- `run_mesh()` function stub in `crates/assay-core/src/orchestrate/mesh.rs` (accepts `OrchestratorConfig` + `SessionRunner` closure, returns `OrchestratorResult`)
- Validation: `depends_on` on sessions in Mesh/Gossip mode emits `tracing::warn` and is ignored

Consumes:
- nothing (first slice)

### S01 → S03

Produces:
- `GossipConfig` type in `assay-types` (`coordinator_interval_secs`) with schema snapshot
- `gossip_config: Option<GossipConfig>` field on `RunManifest`
- `run_gossip()` function stub in `crates/assay-core/src/orchestrate/gossip.rs`

Consumes:
- nothing (first slice)

### S02 → S04

Produces:
- `MeshMemberState` enum: `Alive`, `Suspect`, `Dead` with schema snapshot
- `MeshMemberStatus` struct: `{ name: String, state: MeshMemberState, last_heartbeat_at: Option<DateTime<Utc>> }` with schema snapshot
- `MeshStatus` struct: `{ members: Vec<MeshMemberStatus>, messages_routed: u64 }` — extends `OrchestratorStatus` via `mesh_status: Option<MeshStatus>` field
- `run_mesh()` full implementation: parallel dispatch (no DAG), roster prompt layer injection, `.assay/orchestrator/<run_id>/mesh/<name>/inbox/` and `.../outbox/` directory creation, routing thread polling outboxes and routing to inboxes, heartbeat file polling for membership state
- Integration test: `test_mesh_mode_message_routing` — 2 mock sessions, one writes outbox file, verify it arrives in peer inbox and `mesh_status.messages_routed == 1`

Consumes from S01:
- `OrchestratorMode::Mesh` dispatch routing
- `MeshConfig` type for timing configuration
- `run_mesh()` stub (replaced by full implementation)

### S03 → S04

Produces:
- `KnowledgeEntry` struct: `{ session_name: String, spec: String, gate_pass_count: u32, gate_fail_count: u32, changed_files: Vec<String>, completed_at: DateTime<Utc> }` with schema snapshot
- `KnowledgeManifest` struct: `{ run_id: String, entries: Vec<KnowledgeEntry>, last_updated_at: DateTime<Utc> }` with schema snapshot; persisted to `.assay/orchestrator/<run_id>/gossip/knowledge.json`
- `GossipStatus` struct: `{ sessions_synthesized: u32, knowledge_manifest_path: PathBuf, coordinator_rounds: u32 }` — extends `OrchestratorStatus` via `gossip_status: Option<GossipStatus>` field
- `run_gossip()` full implementation: parallel dispatch (no DAG), knowledge manifest path injected as PromptLayer at launch, coordinator thread watching for session completions and updating `knowledge.json` atomically, `gossip_status` populated in state.json
- Integration test: `test_gossip_mode_knowledge_manifest` — 3 mock sessions, verify knowledge manifest has 3 entries after all complete, `gossip_status.sessions_synthesized == 3`

Consumes from S01:
- `OrchestratorMode::Gossip` dispatch routing
- `GossipConfig` type for coordinator interval configuration
- `run_gossip()` stub (replaced by full implementation)

### S04 consumes from S02 + S03

- `MeshStatus` type for `orchestrate_status` extension
- `GossipStatus` type for `orchestrate_status` extension
- Mode-specific status fields for MCP `orchestrate_status` response
- CLI mode display
