---
id: S03
parent: M008
milestone: M008
provides:
  - plugins/opencode/AGENTS.md with 10 MCP tool names and 5 skill references
  - 5 skill files in plugins/opencode/skills/ (gate-check, spec-show, cycle-status, next-chunk, plan)
  - .gitkeep files removed from plugins/opencode/skills/ and plugins/opencode/agents/
requires:
  - slice: M005/S06
    provides: Codex plugin skill files used as verbatim source
affects:
  - none (standalone — no downstream slices consume OpenCode plugin content)
key_files:
  - plugins/opencode/AGENTS.md
  - plugins/opencode/skills/gate-check.md
  - plugins/opencode/skills/spec-show.md
  - plugins/opencode/skills/cycle-status.md
  - plugins/opencode/skills/next-chunk.md
  - plugins/opencode/skills/plan.md
key_decisions:
  - D119 — OpenCode plugin uses Codex flat-file skill convention
patterns_established:
  - Three-platform plugin parity: Claude Code, Codex, and OpenCode all share identical skill content; only AGENTS.md title heading differs
observability_surfaces:
  - none (pure markdown, no runtime boundary)
drill_down_paths:
  - .kata/milestones/M008/slices/S03/tasks/T01-SUMMARY.md
duration: ~5m
verification_result: passed
completed_at: 2026-03-24
---

# S03: OpenCode Plugin with Full Skill Parity

**Created `plugins/opencode/AGENTS.md` (37 lines, 10 MCP tools) and 5 skill files matching Codex plugin exactly, completing three-platform plugin parity.**

## What Happened

Deleted `.gitkeep` placeholders from `plugins/opencode/skills/` and `plugins/opencode/agents/`. Wrote `plugins/opencode/AGENTS.md` — identical to `plugins/codex/AGENTS.md` with "Codex" replaced by "OpenCode" in the title heading; includes `pr_create` as the 10th MCP tool row. Copied all 5 skill files verbatim from `plugins/codex/skills/` to `plugins/opencode/skills/`: gate-check, spec-show, cycle-status, next-chunk, plan. Content is platform-neutral — no changes needed between Codex and OpenCode.

## Verification

22 structural checks passed:
- 6 file existence checks (AGENTS.md + 5 skills) ✓
- 2 .gitkeep removal checks ✓
- AGENTS.md line count: 37 (≤60 limit) ✓
- 10 MCP tool name presence checks (spec_list, spec_get, gate_run, cycle_status, cycle_advance, chunk_status, milestone_list, milestone_create, spec_create, pr_create) ✓
- No SKILL.md subdirectory files (flat .md format) ✓
- Interview-first pattern in plan.md ✓
- Two null guards in next-chunk.md (active:false + active_chunk_slug) ✓
- cmd editing warning in plan.md ✓
- opencode.json untouched ✓

## Requirements Advanced

- R057 — OpenCode plugin now has AGENTS.md + 5 skills with correct MCP tool references, completing the three-platform plugin parity requirement

## Requirements Validated

- R057 — OpenCode plugin delivered with structural verification proving file existence, correct tool names, skill patterns (interview-first, null guards), and format consistency with Codex plugin

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None. All files delivered exactly as planned.

## Known Limitations

- Skills reference MCP tool names but do not validate that those tools exist at runtime — structural checks only
- opencode.json configuration was pre-existing and not modified; actual OpenCode runtime integration is outside Assay's scope

## Follow-ups

- none

## Files Created/Modified

- `plugins/opencode/AGENTS.md` — workflow guide with 5 skills + 10 MCP tools table (37 lines)
- `plugins/opencode/skills/gate-check.md` — gate check skill (copied from codex)
- `plugins/opencode/skills/spec-show.md` — spec display skill (copied from codex)
- `plugins/opencode/skills/cycle-status.md` — cycle overview skill (copied from codex)
- `plugins/opencode/skills/next-chunk.md` — chunk detail skill with two null guards (copied from codex)
- `plugins/opencode/skills/plan.md` — milestone creation skill with interview-first pattern (copied from codex)
- `plugins/opencode/skills/.gitkeep` — deleted
- `plugins/opencode/agents/.gitkeep` — deleted

## Forward Intelligence

### What the next slice should know
- S03 is fully standalone — no code changes, no compilation, no schema updates. S04 and S05 can proceed independently.

### What's fragile
- nothing — pure markdown with no runtime coupling

### Authoritative diagnostics
- `ls plugins/opencode/skills/` and `wc -l plugins/opencode/AGENTS.md` are the canonical inspection surfaces

### What assumptions changed
- none — S03 delivered exactly as planned with zero deviations
