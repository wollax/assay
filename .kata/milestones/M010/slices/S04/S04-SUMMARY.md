---
id: S04
parent: M010
milestone: M010
provides:
  - plugins/smelt-agent/AGENTS.md (27 lines) — smelt-agent role, skills table, MCP tool table, workflow overview
  - plugins/smelt-agent/skills/run-dispatch.md — RunManifest reading, StateBackendConfig setup, orchestrate_run/run_manifest dispatch
  - plugins/smelt-agent/skills/backend-status.md — orchestrate_status polling, OrchestratorStatus interpretation, CapabilitySet degradation awareness
  - plugins/smelt-agent/skills/peer-message.md — outbox/inbox file convention, roster PromptLayer parsing, message send/receive lifecycle
requires:
  - slice: S02
    provides: "StateBackend API surface, OrchestratorStatus/SessionStatus schemas, MCP tool signatures (orchestrate_run, orchestrate_status, run_manifest)"
  - slice: S03
    provides: "CapabilitySet degradation behavior for backend-status and peer-message skills"
affects: []
key_files:
  - plugins/smelt-agent/AGENTS.md
  - plugins/smelt-agent/skills/run-dispatch.md
  - plugins/smelt-agent/skills/backend-status.md
  - plugins/smelt-agent/skills/peer-message.md
key_decisions:
  - All four files authored in a single pass (same pattern as D082 for Codex S06)
  - Flat .md skill files in plugins/smelt-agent/skills/ following Codex/OpenCode flat-file convention
  - AGENTS.md ≤60 lines with skill table + MCP tool table + workflow section
patterns_established:
  - smelt-agent plugin follows same file structure as claude-code and codex plugins
  - MCP tool names referenced in skills verified against server.rs grep before commit
observability_surfaces:
  - none (documentation only — no runtime signals)
drill_down_paths: []
duration: ~20min
verification_result: passed
completed_at: 2026-03-26
---

# S04: smelt-agent plugin

**`plugins/smelt-agent/` with `AGENTS.md` + 3 skills (run-dispatch, backend-status, peer-message) documenting the backend-aware API surface — R075 validated, `just ready` green.**

## What Happened

All four files written in a single T01 pass, following the D082 pattern (single-context-window authoring when all content fits without ordering dependencies).

**AGENTS.md** (27 lines) describes the smelt-agent role, lists the three skills and their purposes, lists the three relevant MCP tools (`run_manifest`, `orchestrate_run`, `orchestrate_status`) with descriptions, and provides a 5-step workflow overview covering dispatch → monitor → coordinate → report → degrade gracefully.

**skills/run-dispatch.md** covers: locating a RunManifest, configuring `StateBackendConfig` variants (LocalFs / Custom), choosing between `run_manifest` (single session) and `orchestrate_run` (multi-session), dispatching with parameter examples, and notes on `failure_policy` / `merge_strategy` / `mode` options.

**skills/backend-status.md** covers: querying `orchestrate_status` with `run_id`, interpreting all `OrchestratorStatus` fields (phase, sessions, mesh_status, gossip_status), reading `SessionStatus.state` transitions, understanding `MeshStatus.messages_routed` and `GossipStatus.sessions_synthesized`, and identifying CapabilitySet degradation signals from observable behavior.

**skills/peer-message.md** covers: the full outbox/inbox directory layout under `.assay/orchestrator/<run_id>/mesh/`, parsing the `Outbox:` line from the mesh-roster `PromptLayer`, writing message files to `<outbox>/<target>/<name>`, reading and consuming inbox files, checking for messaging capability degradation via `messages_routed == 0`, and at-least-once delivery caveats.

## Verification

- `test -f plugins/smelt-agent/AGENTS.md` — exists ✅
- `test -f plugins/smelt-agent/skills/run-dispatch.md` — exists ✅
- `test -f plugins/smelt-agent/skills/backend-status.md` — exists ✅
- `test -f plugins/smelt-agent/skills/peer-message.md` — exists ✅
- `wc -l plugins/smelt-agent/AGENTS.md` → 27 (≤60) ✅
- `head -2 plugins/smelt-agent/skills/*.md | grep "^name:"` — frontmatter present on all 3 skills ✅
- `grep -c "orchestrate_run\|orchestrate_status\|run_manifest" plugins/smelt-agent/AGENTS.md` → 6 ✅
- `just ready` — green ✅

## Requirements Advanced

- R075 — smelt-agent plugin: fully proven. Four files exist, AGENTS.md ≤60 lines, all skills have valid YAML frontmatter, tool names verified against server.rs.

## Requirements Validated

- R075 — proved by this slice.

## Deviations

- None. All files match plan specification.

## Known Limitations

- UAT verification (a human reads the skills and confirms accuracy) is deferred to end-user adoption. The structural and reference checks (tool names, type names) are machine-verified; content accuracy requires domain expertise review.

## Files Created/Modified

- `plugins/smelt-agent/AGENTS.md` — new: smelt-agent system prompt
- `plugins/smelt-agent/skills/run-dispatch.md` — new: run dispatch skill
- `plugins/smelt-agent/skills/backend-status.md` — new: backend status skill
- `plugins/smelt-agent/skills/peer-message.md` — new: peer message skill
