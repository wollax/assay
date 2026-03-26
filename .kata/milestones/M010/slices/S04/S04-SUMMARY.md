---
id: S04
parent: M010
milestone: M010
provides:
  - plugins/smelt-agent/AGENTS.md — smelt worker system prompt with skills table, MCP tools table, and workflow overview
  - plugins/smelt-agent/skills/run-dispatch.md — skill for dispatching runs via run_manifest and orchestrate_run
  - plugins/smelt-agent/skills/backend-status.md — skill for querying orchestrate_status and interpreting OrchestratorStatus
  - plugins/smelt-agent/skills/peer-message.md — skill for mesh outbox/inbox messaging and gossip knowledge manifest reading
requires:
  - slice: S02
    provides: StateBackend API surface, OrchestratorStatus/SessionStatus/MeshStatus/GossipStatus schemas, RunManifest.state_backend field, mesh outbox/inbox convention
  - slice: S03
    provides: CapabilitySet degradation behavior (supports_messaging, supports_gossip_manifest guards)
affects: []
key_files:
  - plugins/smelt-agent/AGENTS.md
  - plugins/smelt-agent/skills/run-dispatch.md
  - plugins/smelt-agent/skills/backend-status.md
  - plugins/smelt-agent/skills/peer-message.md
key_decisions:
  - "Followed codex plugin format (D082) — skills table + MCP tools table + workflow section in AGENTS.md"
  - "Described file-based outbox/inbox convention rather than non-existent send_message/poll_inbox MCP tools"
  - "Included CapabilitySet degradation awareness in both backend-status and peer-message skills per S03 patterns"
patterns_established:
  - "smelt-agent plugin directory at plugins/smelt-agent/ with AGENTS.md + skills/ subdirectory matching codex/opencode plugin structure"
observability_surfaces:
  - none (pure documentation)
drill_down_paths:
  - .kata/milestones/M010/slices/S04/tasks/T01-SUMMARY.md
duration: 15min
verification_result: passed
completed_at: 2025-07-27T12:00:00Z
---

# S04: smelt-agent plugin

**`plugins/smelt-agent/` created with AGENTS.md and three skills teaching smelt workers the backend-aware Assay API surface for run dispatch, status queries, and peer coordination.**

## What Happened

Created `plugins/smelt-agent/` directory as a single task (T01) following the codex plugin format established in D082. All four files (AGENTS.md + 3 skills) were written in one context window with no inter-file ordering dependencies.

**AGENTS.md** (45 lines, ≤60 limit): Describes the smelt-agent role as an AI worker agent that orchestrates Assay runs on behalf of a smelt controller. Contains a skills table (3 skills), MCP tools table (10 tools: spec_list, spec_get, gate_run, run_manifest, orchestrate_run, orchestrate_status, milestone_list, milestone_get, cycle_status, cycle_advance), a workflow section covering the receive→configure→dispatch→monitor→report lifecycle, and a CapabilitySet awareness section explaining graceful degradation when backends don't support messaging or gossip manifest.

**run-dispatch.md**: Covers RunManifest TOML format (`[[sessions]]` array, `mode` field, `state_backend` field), StateBackendConfig variants (LocalFs and Custom), single-session dispatch via `run_manifest` MCP tool, and multi-session orchestration via `orchestrate_run` with failure_policy, merge_strategy, and conflict_resolution parameters.

**backend-status.md**: Covers `orchestrate_status` MCP tool queries, OrchestratorStatus field interpretation (phase: Running/Completed/PartialFailure/Aborted), SessionStatus states (Pending/Running/Completed/Failed/Skipped), mesh_status and gossip_status sub-fields, CapabilitySet degradation awareness (what warnings mean, how to detect unsupported capabilities), and error handling patterns.

**peer-message.md**: Covers mesh-roster PromptLayer parsing (how to extract peer names and outbox paths), outbox/inbox file messaging convention (write JSON files to `.assay/orchestrator/<run_id>/mesh/<name>/outbox/`, poll `.assay/orchestrator/<run_id>/mesh/<name>/inbox/`), gossip-knowledge-manifest PromptLayer parsing and knowledge.json reading, and CapabilitySet guards for both mesh and gossip modes.

## Verification

- All 4 files exist: `test -f` confirmed for each
- AGENTS.md is 45 lines (≤60 limit): `wc -l` confirmed
- All 3 skills have valid YAML frontmatter with `name` and `description` fields: `head -5` confirmed
- Tool names `orchestrate_run`, `orchestrate_status`, `run_manifest` appear in AGENTS.md (5 references): `grep -c` confirmed
- All MCP tool names match registered tools in server.rs: grep verified
- All type names (OrchestratorStatus, SessionStatus, MeshStatus, GossipStatus, RunManifest, StateBackendConfig, CapabilitySet) exist in assay-types/assay-core: grep verified
- No .gitkeep files in directory
- `just ready` green: all checks passed (fmt + lint + test + deny)

## Requirements Advanced

- R075 — smelt-agent plugin fully delivered: AGENTS.md + 3 skills covering run dispatch, backend status, and peer messaging

## Requirements Validated

- R075 — Plugin exists with AGENTS.md and 3 skills covering all three specified topics; all tool and type names verified correct; `just ready` green

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

Task plan referenced `send_message`/`poll_inbox` as MCP tool names for peer messaging. These don't exist as MCP tools — peer messaging is file-based (outbox/inbox directories). `peer-message.md` correctly describes the actual file-based convention instead of non-existent MCP tools.

## Known Limitations

- Skills describe the S01–S03 API surface; concrete remote backend implementations (LinearBackend, GitHubBackend, SshSyncBackend) will require skill updates in M011+ once those backends exist
- CapabilitySet degradation guidance in the skills is advisory — it describes observed behavior from S03 but cannot be programmatically verified by a skill reader

## Follow-ups

- M011: Update skills when concrete remote backends are implemented to cover backend-specific configuration and error patterns

## Files Created/Modified

- `plugins/smelt-agent/AGENTS.md` — smelt worker system prompt (45 lines)
- `plugins/smelt-agent/skills/run-dispatch.md` — run dispatch skill with RunManifest + orchestrate_run coverage
- `plugins/smelt-agent/skills/backend-status.md` — backend status skill with OrchestratorStatus interpretation
- `plugins/smelt-agent/skills/peer-message.md` — peer messaging skill with mesh outbox/inbox + gossip knowledge manifest

## Forward Intelligence

### What the next slice should know
- This is the final slice of M010; the milestone definition of done is now fully met
- All existing plugins (claude-code, codex, opencode) are at version 0.5.0 — smelt-agent follows the same flat `.md` file convention for skills
- The file-based outbox/inbox convention (not MCP tools) is the actual peer messaging mechanism; any future MCP tool wrapping of this convention should be documented as an addition

### What's fragile
- Skills reference type schemas from assay-types — if type names change in M011+, skills will silently become stale

### Authoritative diagnostics
- `grep -r "orchestrate_run\|orchestrate_status\|run_manifest" plugins/smelt-agent/` — verify tool name accuracy after any server.rs changes
- `just ready` — confirms no regressions from documentation additions

### What assumptions changed
- Original plan assumed `send_message`/`poll_inbox` were MCP tools — they are backend trait methods only; actual peer messaging is file-based
