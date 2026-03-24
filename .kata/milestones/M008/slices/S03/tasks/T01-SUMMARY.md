---
id: T01
parent: S03
milestone: M008
provides:
  - plugins/opencode/AGENTS.md with all 10 MCP tool names
  - 5 skill files in plugins/opencode/skills/
  - .gitkeep files removed from skills/ and agents/
key_files:
  - plugins/opencode/AGENTS.md
  - plugins/opencode/skills/gate-check.md
  - plugins/opencode/skills/spec-show.md
  - plugins/opencode/skills/cycle-status.md
  - plugins/opencode/skills/next-chunk.md
  - plugins/opencode/skills/plan.md
key_decisions:
  - none (pure copy/adapt from codex plugin per plan)
patterns_established:
  - OpenCode plugin mirrors Codex plugin content exactly; only the title heading differs
observability_surfaces:
  - none (pure markdown, no runtime boundary)
duration: ~5m
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T01: Author AGENTS.md and all 5 skill files for the OpenCode plugin

**Created `plugins/opencode/AGENTS.md` (37 lines, 10 MCP tools) and 5 skill files copied verbatim from the Codex plugin, completing the OpenCode plugin content.**

## What Happened

1. Deleted `.gitkeep` placeholders from `plugins/opencode/skills/` and `plugins/opencode/agents/`.
2. Wrote `plugins/opencode/AGENTS.md` — identical to `plugins/codex/AGENTS.md` with "Codex" replaced by "OpenCode" in the first heading, plus `pr_create` added as the 10th MCP tool row.
3. Copied all 5 skill files unchanged from `plugins/codex/skills/` to `plugins/opencode/skills/`: `gate-check.md`, `spec-show.md`, `cycle-status.md`, `next-chunk.md`, `plan.md`.

## Verification

All 22 checks passed:
- File existence: all 6 target files present
- `.gitkeep` removal: both deleted
- AGENTS.md line count: 37 (≤60 ✓)
- All 10 MCP tool names present in AGENTS.md: spec_list, spec_get, gate_run, cycle_status, cycle_advance, chunk_status, milestone_list, milestone_create, spec_create, pr_create ✓
- No `SKILL.md` subdirectory files (flat .md format) ✓
- `"before any MCP tool"` pattern in plan.md ✓
- `{"active":false}` guard in next-chunk.md ✓
- `active_chunk_slug` null guard in next-chunk.md ✓
- `cmd` editing warning in plan.md ✓
- `opencode.json` untouched ✓

## Diagnostics

Pure markdown — no runtime signals. Inspect via `ls plugins/opencode/skills/` and `wc -l plugins/opencode/AGENTS.md`.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `plugins/opencode/AGENTS.md` — workflow guide with 5 skills + 10 MCP tools table (37 lines)
- `plugins/opencode/skills/gate-check.md` — gate check skill (copied from codex)
- `plugins/opencode/skills/spec-show.md` — spec display skill (copied from codex)
- `plugins/opencode/skills/cycle-status.md` — cycle overview skill (copied from codex)
- `plugins/opencode/skills/next-chunk.md` — chunk detail skill with two null guards (copied from codex)
- `plugins/opencode/skills/plan.md` — milestone creation skill with interview-first pattern (copied from codex)
- `plugins/opencode/skills/.gitkeep` — deleted
- `plugins/opencode/agents/.gitkeep` — deleted
