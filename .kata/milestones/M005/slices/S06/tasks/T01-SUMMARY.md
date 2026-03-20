---
id: T01
parent: S06
milestone: M005
provides:
  - plugins/codex/AGENTS.md — 34-line workflow reference with skills table, MCP tools table, and numbered workflow steps
  - plugins/codex/skills/gate-check.md — ported from claude-code; uses spec_list + gate_run
  - plugins/codex/skills/spec-show.md — ported from claude-code; uses spec_list + spec_get
  - plugins/codex/skills/cycle-status.md — new; overview of active milestone progress; handles active:false and has_history:false
  - plugins/codex/skills/next-chunk.md — new; active chunk detail with full criteria list; handles active:false and has_history:false
  - plugins/codex/skills/plan.md — new; interview-first milestone creation; collects all inputs before any MCP call
key_files:
  - plugins/codex/AGENTS.md
  - plugins/codex/skills/gate-check.md
  - plugins/codex/skills/spec-show.md
  - plugins/codex/skills/cycle-status.md
  - plugins/codex/skills/next-chunk.md
  - plugins/codex/skills/plan.md
key_decisions:
  - "cycle-status and next-chunk kept as separate skills per S06-CONTEXT: cycle-status = overview first, next-chunk = chunk detail + criteria list"
  - "plan skill strictly interview-first: all 3 input-gathering steps complete before any MCP tool call (milestone_list check in step 4)"
  - "AGENTS.md tables only 8 most workflow-relevant MCP tools to stay under 60-line limit (actual: 34 lines)"
patterns_established:
  - "Codex skill files are flat .md files with YAML frontmatter (name + description) matching claude-code SKILL.md convention"
  - "Skills that call cycle_status must explicitly handle {\"active\":false} inline with user-facing guidance"
  - "Skills that call chunk_status must not assume passed/failed fields — check has_history first"
observability_surfaces:
  - "wc -l plugins/codex/AGENTS.md — verify line count stays ≤60"
  - "cat plugins/codex/skills/<name>.md — inspect skill content"
  - "MCP tool errors surface directly in Codex when a skill fails"
duration: 15min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T01: Write AGENTS.md and all 5 skill files

**Replaced 3-line AGENTS.md stub with a 34-line workflow reference and authored 5 self-contained skill files covering the full Assay development cycle for Codex users.**

## What Happened

All 6 content files were authored in a single pass in dependency order: AGENTS.md first as the anchor reference, then skills gate-check → spec-show → cycle-status → next-chunk → plan.

`AGENTS.md` (34 lines, well under 60) contains three sections: a one-paragraph workflow intro, a skills table listing all 5 commands with descriptions, an MCP tools table with the 8 most workflow-relevant tools, and 5 numbered workflow steps (plan → read → implement → gate-check → advance).

`gate-check.md` and `spec-show.md` are direct ports from `plugins/claude-code/skills/*/SKILL.md` with frontmatter added and format adapted from subdirectory SKILL.md to flat `.md` file. No semantic changes.

`cycle-status.md` is a new overview skill. It calls `cycle_status` first; if `{"active":false}` it stops with guidance to run `/assay:plan`. If active, it calls `chunk_status` for the active chunk and displays a concise 6-line-or-fewer table with milestone name/phase, chunk progress (X/N), active chunk slug, and latest gate counts. Handles `has_history:false` explicitly.

`next-chunk.md` is a new chunk-detail skill. It also calls `cycle_status` first (same active:false handling), then `chunk_status`, then `spec_get` to load the full criteria list. Output is chunk slug/name + gate status + full criteria grouped by executable vs descriptive — giving the implementing agent a complete implementation target.

`plan.md` is a new interview-first skill. Steps 1–3 collect milestone goal/name/slug, chunk list, and criteria per chunk entirely through conversation. Step 4 is the first MCP call (`milestone_list` to check for slug collision). Steps 5–6 call `milestone_create` and `spec_create`. Output confirms what was created and includes the mandatory warning that `cmd` fields must be manually added to generated `gates.toml` files before gates are runnable (D076).

`plugins/codex/skills/.gitkeep` was deleted as required.

## Verification

All verification checks passed (27/27):
- `wc -l plugins/codex/AGENTS.md` → 34 lines (≤60 ✓)
- All 5 skill files exist
- `.gitkeep` removed
- Tool name correctness: gate_run, spec_get, cycle_status+chunk_status, cycle_status+spec_get, milestone_create+spec_create
- Graceful degradation: active:false handling in cycle-status and next-chunk
- Interview-first: plan collects goal/chunks/criteria before MCP calls
- cmd editing note present in plan
- AGENTS.md mentions all 5 skills

## Diagnostics

Pure markdown — no runtime state. Inspect with:
- `wc -l plugins/codex/AGENTS.md` — line count check
- `cat plugins/codex/skills/<name>.md` — skill content inspection
- MCP tool errors surface directly in Codex when a skill fails; skill files handle the known edge cases (active:false, has_history:false) to avoid silent failure

## Deviations

None. S06 is a pure-markdown slice — all deliverables authored exactly as planned.

## Known Issues

None.

## Files Created/Modified

- `plugins/codex/AGENTS.md` — replaced 3-line stub with 34-line workflow reference
- `plugins/codex/skills/gate-check.md` — new: ported from claude-code gate-check
- `plugins/codex/skills/spec-show.md` — new: ported from claude-code spec-show
- `plugins/codex/skills/cycle-status.md` — new: milestone overview skill
- `plugins/codex/skills/next-chunk.md` — new: active chunk detail skill
- `plugins/codex/skills/plan.md` — new: interview-first milestone creation skill
- `plugins/codex/skills/.gitkeep` — deleted
