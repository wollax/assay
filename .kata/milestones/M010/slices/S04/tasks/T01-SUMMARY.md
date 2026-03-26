---
id: T01
parent: S04
milestone: M010
provides:
  - plugins/smelt-agent/AGENTS.md — smelt worker system prompt with skills table and MCP tools table
  - plugins/smelt-agent/skills/run-dispatch.md — skill for dispatching runs via run_manifest and orchestrate_run
  - plugins/smelt-agent/skills/backend-status.md — skill for querying orchestrate_status and interpreting OrchestratorStatus
  - plugins/smelt-agent/skills/peer-message.md — skill for mesh outbox/inbox messaging and gossip knowledge manifest
key_files:
  - plugins/smelt-agent/AGENTS.md
  - plugins/smelt-agent/skills/run-dispatch.md
  - plugins/smelt-agent/skills/backend-status.md
  - plugins/smelt-agent/skills/peer-message.md
key_decisions:
  - "Followed codex plugin format (D082) — skills table + MCP tools table + workflow section in AGENTS.md"
  - "Included CapabilitySet degradation awareness in both backend-status and peer-message skills per S03 patterns"
patterns_established:
  - "smelt-agent plugin directory at plugins/smelt-agent/ with AGENTS.md + skills/ subdirectory"
observability_surfaces:
  - none (pure documentation)
duration: 15min
verification_result: passed
completed_at: 2025-07-27T12:00:00Z
blocker_discovered: false
---

# T01: AGENTS.md and all three skills

**Created smelt-agent plugin with system prompt and three skills covering run dispatch, backend status, and peer messaging**

## What Happened

Created `plugins/smelt-agent/` directory with four files following the codex plugin format (D082). AGENTS.md (45 lines) provides the smelt worker role description, a skills table listing all three skills, an MCP tools table with 9 tools, a workflow section explaining the receive→configure→dispatch→monitor→report lifecycle, and a CapabilitySet awareness section explaining graceful degradation.

Three skills were written with YAML frontmatter and step-by-step instructions:
- `run-dispatch.md` — covers RunManifest TOML format (`[[sessions]]`, `mode`, `state_backend`), StateBackendConfig variants (LocalFs/Custom), single-session dispatch via `run_manifest`, and multi-session orchestration via `orchestrate_run` with all parameters (failure_policy, merge_strategy, conflict_resolution)
- `backend-status.md` — covers `orchestrate_status` queries, OrchestratorStatus field interpretation (phase: Running/Completed/PartialFailure/Aborted), SessionStatus states, mesh_status and gossip_status sub-fields, CapabilitySet degradation awareness, and error handling
- `peer-message.md` — covers mesh-roster PromptLayer parsing, outbox/inbox file messaging convention, gossip-knowledge-manifest PromptLayer and knowledge.json reading, and CapabilitySet guards for both modes

## Verification

- All 4 files exist: confirmed via `test -f`
- AGENTS.md is 45 lines (≤60 limit): confirmed via `wc -l`
- All 3 skills have valid YAML frontmatter with `name` and `description` fields: confirmed via `head -3`
- Tool names `orchestrate_run`, `orchestrate_status`, `run_manifest` appear in AGENTS.md (5 references): confirmed via `grep -c`
- All MCP tool names match registered tools in server.rs: confirmed via grep against `fn spec_list`, `fn spec_get`, `fn gate_run`, `fn orchestrate_run`, etc.
- All type names (OrchestratorStatus, SessionStatus, MeshStatus, GossipStatus, RunManifest, StateBackendConfig, CapabilitySet) exist in assay-types/assay-core: confirmed via grep
- No .gitkeep files: confirmed via `find`
- `just ready` green: all checks passed

Slice verification (all pass — this is the only task):
- ✓ All 4 files exist
- ✓ AGENTS.md ≤60 lines (45)
- ✓ All skills have YAML frontmatter
- ✓ Tool names present in AGENTS.md
- ✓ `just ready` green

## Diagnostics

None — pure documentation, no runtime inspection needed.

## Deviations

- Task plan referenced `send_message`/`poll_inbox` tool names for peer messaging; these don't exist as MCP tools. Instead, peer-message.md describes the file-based outbox/inbox convention (write files to outbox directory, read from inbox directory) which is the actual mesh messaging mechanism. The gossip mode uses knowledge manifest file reading instead.

## Known Issues

None.

## Files Created/Modified

- `plugins/smelt-agent/AGENTS.md` — smelt worker system prompt (45 lines)
- `plugins/smelt-agent/skills/run-dispatch.md` — run dispatch skill with RunManifest + orchestrate_run coverage
- `plugins/smelt-agent/skills/backend-status.md` — backend status skill with OrchestratorStatus interpretation
- `plugins/smelt-agent/skills/peer-message.md` — peer messaging skill with mesh outbox/inbox + gossip knowledge manifest
