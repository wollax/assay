# S04: smelt-agent plugin

**Goal:** A `plugins/smelt-agent/` directory exists with AGENTS.md and three skills that teach a smelt worker agent how to use the backend-aware Assay API surface: dispatching runs, querying backend status, and using peer messaging.
**Demo:** A human reads the AGENTS.md and skills and can follow the instructions to operate Assay orchestration from a smelt worker context. The skills reference real MCP tool signatures and real type schemas from S01–S02.

## Must-Haves

- `plugins/smelt-agent/AGENTS.md` exists with ≤60 lines, describing the smelt-agent role, listing available skills and MCP tools in a table, and providing a workflow overview
- `plugins/smelt-agent/skills/run-dispatch.md` exists with valid YAML frontmatter (`name`, `description`), covering how to read a `RunManifest`, configure a `StateBackendConfig`, and dispatch a run via `orchestrate_run` or `run_manifest` MCP tools
- `plugins/smelt-agent/skills/backend-status.md` exists with valid YAML frontmatter, covering how to call `orchestrate_status`, interpret `OrchestratorStatus` fields (phase, session states, mesh_status, gossip_status), and check `CapabilitySet` degradation
- `plugins/smelt-agent/skills/peer-message.md` exists with valid YAML frontmatter, covering how to use `send_message`/`poll_inbox` for agent-to-agent coordination, the outbox/inbox file convention, and the mesh roster PromptLayer format
- All MCP tool names referenced in the skills match actual registered tool names (verified by grep against `server.rs`)
- All type/schema names referenced match actual types in `assay-types` (verified by grep)
- No `.gitkeep` files in the directory
- `just ready` green (no Rust changes, but confirm no regressions)
- R075 is proven: plugin exists with AGENTS.md + 3 skills covering the three specified topics

## Proof Level

- This slice proves: documentation completeness (human-verified)
- Real runtime required: no (pure markdown, no code changes)
- Human/UAT required: yes (a human reads and confirms accuracy)

## Verification

- `test -f plugins/smelt-agent/AGENTS.md` — file exists
- `test -f plugins/smelt-agent/skills/run-dispatch.md` — file exists
- `test -f plugins/smelt-agent/skills/backend-status.md` — file exists
- `test -f plugins/smelt-agent/skills/peer-message.md` — file exists
- `wc -l plugins/smelt-agent/AGENTS.md` — ≤60 lines
- `head -5 plugins/smelt-agent/skills/run-dispatch.md | grep "^name:"` — YAML frontmatter present
- `head -5 plugins/smelt-agent/skills/backend-status.md | grep "^name:"` — YAML frontmatter present
- `head -5 plugins/smelt-agent/skills/peer-message.md | grep "^name:"` — YAML frontmatter present
- `grep -c "orchestrate_run\|orchestrate_status\|run_manifest" plugins/smelt-agent/AGENTS.md` — tool names present
- `just ready` — green

## Observability / Diagnostics

- Runtime signals: none (pure documentation)
- Inspection surfaces: none
- Failure visibility: none
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `StateBackend` trait API (S01), `StateBackendConfig` (S01), `OrchestratorConfig.backend` (S02), `CapabilitySet` (S01), `RunManifest.state_backend` field (S02), MCP tools `orchestrate_run`/`orchestrate_status`/`run_manifest` (M002/S06), mesh outbox/inbox convention (M004/S02), gossip knowledge manifest convention (M004/S03), capability degradation behavior (S03)
- New wiring introduced in this slice: none (documentation only — no code changes)
- What remains before the milestone is truly usable end-to-end: nothing — S04 is the final slice; after this, the milestone definition of done is met

## Tasks

- [x] **T01: AGENTS.md and all three skills** `est:25m`
  - Why: S04 is pure markdown with confirmed MCP tool signatures from S01–S02 and established plugin conventions from codex/opencode plugins. All 4 files (AGENTS.md + 3 skills) fit comfortably in one context window with no compilation, no schema changes, and no inter-file dependencies that require ordering. Splitting would add overhead for zero benefit (same pattern as D082).
  - Files: `plugins/smelt-agent/AGENTS.md`, `plugins/smelt-agent/skills/run-dispatch.md`, `plugins/smelt-agent/skills/backend-status.md`, `plugins/smelt-agent/skills/peer-message.md`
  - Do: Create `plugins/smelt-agent/` directory. Write AGENTS.md following the codex plugin format (≤60 lines, skill table + MCP tool table + workflow summary). Write `run-dispatch.md` with YAML frontmatter and steps for RunManifest reading, StateBackendConfig setup, and orchestrate_run/run_manifest dispatch. Write `backend-status.md` with steps for orchestrate_status queries, OrchestratorStatus interpretation, and CapabilitySet degradation awareness. Write `peer-message.md` with steps for mesh send_message/poll_inbox, outbox/inbox convention, and roster PromptLayer parsing. Verify all tool names against server.rs grep. Verify all type names against assay-types grep. Run `just ready` to confirm no regressions.
  - Verify: All 4 files exist; AGENTS.md ≤60 lines; all skills have YAML frontmatter; tool names match server.rs; `just ready` green
  - Done when: `plugins/smelt-agent/` has AGENTS.md + 3 skills, all referencing correct tool and type names, `just ready` green

## Files Likely Touched

- `plugins/smelt-agent/AGENTS.md`
- `plugins/smelt-agent/skills/run-dispatch.md`
- `plugins/smelt-agent/skills/backend-status.md`
- `plugins/smelt-agent/skills/peer-message.md`
