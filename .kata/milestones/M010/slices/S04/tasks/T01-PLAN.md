---
estimated_steps: 5
estimated_files: 4
---

# T01: AGENTS.md and all three skills

**Slice:** S04 — smelt-agent plugin
**Milestone:** M010

## Description

Create the `plugins/smelt-agent/` plugin directory with AGENTS.md (system prompt for smelt worker agents) and three skills: `run-dispatch.md`, `backend-status.md`, and `peer-message.md`. These teach a smelt worker agent how to use Assay's backend-aware API surface for orchestration dispatch, status queries, and peer messaging. All four files are pure markdown — no Rust code changes.

## Steps

1. Create the `plugins/smelt-agent/skills/` directory structure. Write `AGENTS.md` following the established codex plugin format (D082, D048 patterns): title heading, one-paragraph role description, skills table (3 entries), MCP tools table (listing `orchestrate_run`, `orchestrate_status`, `run_manifest`, `spec_list`, `spec_get`, `gate_run`, `cycle_status`, `cycle_advance`, `chunk_status`), and a workflow section explaining the smelt worker lifecycle (receive manifest → configure backend → dispatch run → monitor status → report results). Keep ≤60 lines.

2. Write `plugins/smelt-agent/skills/run-dispatch.md` with YAML frontmatter (`name: run-dispatch`, `description` explaining when to use it). Steps: (a) Read the RunManifest TOML file — explain `[[sessions]]` array, `mode` field (dag/mesh/gossip), and `state_backend` field (optional, defaults to LocalFs). (b) If `state_backend` is set to `Custom`, explain how the worker should configure the backend. (c) For single-session work, use `run_manifest` tool with `manifest_path`. (d) For multi-session orchestration, use `orchestrate_run` tool with `manifest_path`, optional `failure_policy`, `merge_strategy`, `conflict_resolution`, `max_concurrency` params. (e) Capture the `run_id` from the response for subsequent status queries.

3. Write `plugins/smelt-agent/skills/backend-status.md` with YAML frontmatter (`name: backend-status`, `description` explaining when to use it). Steps: (a) Call `orchestrate_status` with the `run_id` from a prior `orchestrate_run` invocation. (b) Interpret the response: `OrchestratorStatus.phase` (Preparing/Running/MergePhase/Complete/Failed), `session_statuses[]` (each has `state`: Pending/Running/Success/Failed/Skipped), `mesh_status` (if present: `messages_routed`, member states), `gossip_status` (if present: `sessions_synthesized`, `coordinator_rounds`). (c) Explain CapabilitySet awareness: if the backend has `supports_messaging: false`, mesh_status may show zero routed messages — this is expected degradation, not failure. (d) Error handling: if `orchestrate_status` returns an error, the run_id may be invalid or the run may not have persisted state yet.

4. Write `plugins/smelt-agent/skills/peer-message.md` with YAML frontmatter (`name: peer-message`, `description` explaining when to use it). Steps: (a) In Mesh mode, each session receives a "mesh-roster" PromptLayer listing peers and inbox paths. Parse the roster to find `Outbox: <path>` line and peer inbox paths. (b) To send a message: write a file to `<outbox>/<target-name>/<filename>`. The routing thread moves it to the target's inbox. (c) To receive messages: read files from your inbox directory. (d) Message format is freeform (plain text or JSON). (e) Note: messaging requires `supports_messaging: true` on the backend; if the backend doesn't support messaging, the routing thread is not running and messages won't be delivered — check the CapabilitySet first. (f) In Gossip mode, there is no direct messaging — instead, read the knowledge manifest file (path in the "gossip-knowledge-manifest" PromptLayer) to discover what other sessions have completed.

5. Verify: grep `server.rs` for all tool names referenced in the skills. Grep `assay-types` for all type names (OrchestratorStatus, SessionStatus, MeshStatus, GossipStatus, RunManifest, StateBackendConfig, CapabilitySet). Confirm AGENTS.md ≤60 lines. Confirm all 3 skills have valid YAML frontmatter. Run `just ready` to confirm no regressions.

## Must-Haves

- [ ] `plugins/smelt-agent/AGENTS.md` exists, ≤60 lines, with skills table and MCP tools table
- [ ] `plugins/smelt-agent/skills/run-dispatch.md` exists with valid YAML frontmatter, covers RunManifest + state_backend config + orchestrate_run/run_manifest dispatch
- [ ] `plugins/smelt-agent/skills/backend-status.md` exists with valid YAML frontmatter, covers orchestrate_status + OrchestratorStatus interpretation + CapabilitySet degradation awareness
- [ ] `plugins/smelt-agent/skills/peer-message.md` exists with valid YAML frontmatter, covers mesh outbox/inbox messaging + gossip knowledge manifest reading + CapabilitySet guard
- [ ] All MCP tool names in the skills match actual registered tools in server.rs
- [ ] All type/schema names match actual types in assay-types
- [ ] No `.gitkeep` files in the directory
- [ ] `just ready` green

## Verification

- `test -f plugins/smelt-agent/AGENTS.md && test -f plugins/smelt-agent/skills/run-dispatch.md && test -f plugins/smelt-agent/skills/backend-status.md && test -f plugins/smelt-agent/skills/peer-message.md && echo "all exist"` — prints "all exist"
- `wc -l < plugins/smelt-agent/AGENTS.md` — ≤60
- `head -3 plugins/smelt-agent/skills/run-dispatch.md` — shows `---` then `name: run-dispatch`
- `head -3 plugins/smelt-agent/skills/backend-status.md` — shows `---` then `name: backend-status`
- `head -3 plugins/smelt-agent/skills/peer-message.md` — shows `---` then `name: peer-message`
- `grep -l "orchestrate_run" plugins/smelt-agent/AGENTS.md plugins/smelt-agent/skills/run-dispatch.md` — both files reference the tool
- `grep -l "orchestrate_status" plugins/smelt-agent/skills/backend-status.md` — tool referenced
- `just ready` — green

## Observability Impact

- Signals added/changed: None (documentation only)
- How a future agent inspects this: read the skill files; no runtime inspection needed
- Failure state exposed: None

## Inputs

- `plugins/codex/AGENTS.md` — format model for AGENTS.md (≤60 lines, skill table + MCP tool table)
- `plugins/codex/skills/*.md` — format model for skills (YAML frontmatter + steps)
- `crates/assay-mcp/src/server.rs` — authoritative list of MCP tool names and params
- `crates/assay-types/src/orchestrate.rs` — OrchestratorStatus, SessionStatus, MeshStatus, GossipStatus types
- `crates/assay-types/src/manifest.rs` — RunManifest type
- `crates/assay-types/src/state_backend.rs` — StateBackendConfig type
- `crates/assay-core/src/state_backend.rs` — StateBackend trait, CapabilitySet
- S01 summary: StateBackend trait with 7 methods, CapabilitySet with 4 flags, StateBackendConfig enum
- S02 summary: RunManifest.state_backend field, OrchestratorConfig.backend as Arc<dyn StateBackend>, all writes routed through backend
- S03 (in progress): CapabilitySet degradation — mesh skips routing, gossip skips manifest when capabilities absent

## Expected Output

- `plugins/smelt-agent/AGENTS.md` — smelt worker system prompt
- `plugins/smelt-agent/skills/run-dispatch.md` — run dispatch skill
- `plugins/smelt-agent/skills/backend-status.md` — backend status skill
- `plugins/smelt-agent/skills/peer-message.md` — peer messaging skill
