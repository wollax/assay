# M004: Coordination Modes — Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

## Project Description

Assay is a spec-driven quality gate system for AI coding agents. Agents write code in isolated git worktrees; Assay evaluates output against structured specs using dual-track gates, manages session lifecycle, and orchestrates the merge-back pipeline. Consumed via MCP (22 tools), CLI, and TUI skeleton.

## Why This Milestone

Assay currently has one coordination pattern: DAG-based parallel execution where sessions declare explicit `depends_on` dependencies. This works well for structured, dependency-aware work — but two coordination patterns used widely in multi-agent systems (Mesh and Gossip) are absent.

Mesh enables agents to work independently while having the option to message peers via a shared file-based channel — useful when multiple agents are working on loosely related specs that may benefit from ad-hoc coordination without requiring explicit up-front dependency declarations.

Gossip enables emergent cross-pollination: a coordinator synthesizes completed sessions' gate results and diffs into a knowledge manifest that still-running sessions can read — useful when agents are exploring related problem spaces and would benefit from knowing what others have already done.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Set `mode = "mesh"` in a `RunManifest` TOML file and launch `assay run` — sessions start in parallel, each receives a roster prompt layer listing peers and their inbox paths, agents can write message files to their outbox and the orchestrator routes them to target inboxes
- Set `mode = "gossip"` in a `RunManifest` TOML file and launch `assay run` — sessions start in parallel with the knowledge manifest path in their prompt layer; as sessions complete, the coordinator updates the manifest with gate results and diffs; still-running sessions can read the manifest during execution
- Use `orchestrate_status` (MCP) or CLI to inspect mode-specific state: mesh membership (alive/suspect/dead) or gossip coordinator state (sessions synthesized, knowledge manifest path)
- Continue using existing manifests without a `mode` field — they default to `dag` and all existing behavior is preserved

### Entry point / environment

- Entry point: `assay run <manifest.toml>` (CLI) and `orchestrate_run` MCP tool
- Environment: local dev (single machine, multiple worktrees)
- Live dependencies involved: `claude -p` subprocess (agent), git CLI (worktree ops)

## Completion Class

- Contract complete means: unit + integration tests prove mode dispatch, roster injection, message routing, coordinator synthesis, and knowledge manifest persistence with mock session runners
- Integration complete means: `mode = "mesh"` and `mode = "gossip"` in real manifest TOML files parse, dispatch, and produce correct state.json + mesh/ or gossip/ directories
- Operational complete means: existing DAG runs are unaffected (all 1222+ tests still pass)

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `just ready` passes with all new tests green and 0 warnings (lint, fmt, test, deny)
- A manifest with `mode = "mesh"` launches sessions with correct roster prompt layers; orchestrator message routing is exercised by integration test with mock runners writing outbox files
- A manifest with `mode = "gossip"` launches sessions with knowledge manifest path in prompt layers; coordinator updates manifest as mock sessions complete; `orchestrate_status` returns `gossip_status` with sessions synthesized count
- An existing DAG manifest (no mode field) runs identically to M003 behavior — no regressions
- Schema snapshots for `OrchestratorMode`, `MeshConfig`, `GossipConfig`, and `KnowledgeManifest` are locked

## Risks and Unknowns

- **Concurrent background thread in thread::scope** — Mesh mode needs a message-routing thread running alongside session worker threads within `std::thread::scope`. The scope's lifetime guarantees are strong but composing multiple background threads with the existing condvar dispatch loop requires care. Risk: medium — retire in S02 by building the minimal viable routing thread.
- **Prompt layer injection at launch vs mid-run** — Gossip knowledge manifest injection happens at session launch (the path is injected even if the manifest is empty). Sessions decide when (if ever) to read the file during execution. This is one-directional and simple; the risk is that the injected path is not useful if all sessions start at the same time. In practice, even a partially populated manifest is valuable. Risk: low.
- **Schema backward compatibility** — `RunManifest` already has a locked schema snapshot. Adding `mode` with serde default means existing serialized manifests remain valid. Must verify the schema snapshot update passes the snapshot tests. Risk: low.
- **SWIM heartbeat vs agent process lifecycle** — Agents are not long-running servers; they run until the spec is evaluated and exit. "Heartbeat" files must be understood as "agent wrote this during execution" markers, not truly periodic signals. Suspect/dead detection must account for agents that simply completed rather than crashed. Risk: medium — design to check session run state before classifying as dead.

## Existing Codebase / Prior Art

- `crates/assay-types/src/manifest.rs` — `RunManifest` and `ManifestSession`; adding `mode: OrchestratorMode` here with `serde(default)`
- `crates/assay-types/src/orchestrate.rs` — `OrchestratorStatus`, `SessionStatus`, `MeshStatus`/`GossipStatus` go here
- `crates/assay-core/src/orchestrate/executor.rs` — `run_orchestrated()` is the DAG executor; mode dispatch routes here or to new `mesh.rs`/`gossip.rs`
- `crates/assay-core/src/orchestrate/mod.rs` — register `pub mod mesh` and `pub mod gossip`
- `crates/assay-core/src/orchestrate/dag.rs` — DAG + Kahn's algorithm; Mesh/Gossip do NOT use this
- `crates/assay-types/tests/schema_snapshots.rs` — where new schema snapshots must be added and locked
- `crates/assay-mcp/src/server.rs` — `orchestrate_run` and `orchestrate_status` entry points; `mode` parameter must surface here
- `crates/assay-cli/src/commands/run.rs` — CLI `assay run`; mode surfacing in output

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

Key decisions relevant to M004:
- **D001** — Zero-trait convention: closures, not traits. All mode executors follow this — no `Executor` trait.
- **D004** — TOML with `[[sessions]]` array; manifest format is stable; `mode` is a top-level scalar field.
- **D005** — MCP tools are additive only. `orchestrate_run` and `orchestrate_status` can gain new optional fields; existing fields are immutable.
- **D009** — JSON file-per-record persistence. Mesh and Gossip state persists to `.assay/orchestrator/<run_id>/` alongside existing `state.json`.
- **D017** — `std::thread::scope` threading model. Mesh routing thread lives within the same scope as session worker threads.
- **D022** — Orchestrator state persistence to `.assay/orchestrator/<run_id>/state.json`. Mode-specific state goes in sibling files (`mesh/state.json`, `gossip/knowledge.json`).
- **D032** — `SessionRunner` as closure parameter for testability. Mesh and Gossip executors must also accept a `&SessionRunner` so they can be tested with mock runners.

## Relevant Requirements

- R034 — Mode selection: `mode` field on RunManifest, dispatch routing, schema snapshot
- R035 — Mesh mode: parallel executor + roster injection
- R036 — Mesh peer messaging: inbox/outbox directories + message routing + heartbeats
- R037 — Gossip mode: parallel executor + coordinator synthesis thread
- R038 — Gossip knowledge manifest: injection at launch, atomic update on completion

## Scope

### In Scope

- `OrchestratorMode` enum (`dag`, `mesh`, `gossip`) with schema snapshot
- `mode` field on `RunManifest` with serde default (`dag`)
- Mode dispatch in orchestration entry point
- Mesh executor: parallel launch, roster prompt layer, inbox/outbox directory structure, message routing thread, SWIM-inspired membership (alive/suspect/dead), heartbeat files
- Gossip executor: parallel launch, coordinator thread, knowledge manifest (`KnowledgeManifest` type + schema snapshot), knowledge manifest path in prompt layers
- `orchestrate_status` extensions: `mesh_status` and `gossip_status` optional fields
- `MeshConfig` and `GossipConfig` optional config on RunManifest (heartbeat_interval, suspect_timeout, dead_timeout for Mesh; coordinator_interval for Gossip)
- Integration tests using mock session runners (consistent with D032)
- CLI `assay run` mode display in output

### Out of Scope / Non-Goals

- Real Claude invocations in integration tests (mock runners only — consistent with M002/M003 pattern)
- Network-based peer messaging (file-based only — no sockets, gRPC, or message queues)
- CRDT-backed knowledge stores with vector clocks (Cortex's gossip uses CRDTs; Assay uses a simple coordinator-assembled JSON manifest)
- OpenTelemetry instrumentation (still deferred — R027)
- TUI mode-specific views
- Mode-specific gate evaluation differences (all modes use the same pipeline stages)

## Technical Constraints

- Zero-trait convention (D001): all mode executors are free functions, not trait implementations
- Sync threading (D017): `std::thread::scope`, no async
- Additive MCP (D005): existing `orchestrate_run` and `orchestrate_status` signatures must not break
- `deny_unknown_fields` on persisted types: all new types need it; new fields on existing types need `serde(default)`
- Schema snapshots: all new public types in assay-types that may be persisted or transmitted need schema snapshots locked

## Integration Points

- `assay-types` — `RunManifest`, `OrchestratorStatus`, new mode types (`OrchestratorMode`, `MeshConfig`, `GossipConfig`, `MeshStatus`, `GossipStatus`, `KnowledgeManifest`, `KnowledgeEntry`)
- `assay-core::orchestrate` — new `mesh.rs` and `gossip.rs` modules, mode dispatch in entry point
- `assay-mcp::server` — `orchestrate_run` gains `mode` parameter; `orchestrate_status` returns mode-specific status
- `assay-cli::commands::run` — mode displayed in run output
- `assay-harness::prompt` — `build_prompt()` must produce valid output when PromptLayer with Mesh roster or Gossip manifest path is included

## Open Questions

- Heartbeat frequency vs agent execution time: typical `claude -p` runs take 30s–10min. Suspect/dead timeouts should be on the order of minutes, not seconds. Propose defaults: heartbeat every 30s, suspect after 5min silence, dead after 10min. Planning agents can override via `MeshConfig`.
- Message format: no schema enforced on outbox message files — they're agent-authored JSON or text. The routing layer passes them through without interpretation. Agents decide how to parse inbox messages.
