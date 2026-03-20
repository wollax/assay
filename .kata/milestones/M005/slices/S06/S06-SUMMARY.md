---
id: S06
parent: M005
milestone: M005
provides:
  - plugins/codex/AGENTS.md — 34-line workflow reference with skills table, MCP tools table, and numbered workflow steps
  - plugins/codex/skills/gate-check.md — ported from claude-code; calls spec_list + gate_run
  - plugins/codex/skills/spec-show.md — ported from claude-code; calls spec_list + spec_get
  - plugins/codex/skills/cycle-status.md — new; milestone overview with active chunk progress; handles active:false and has_history:false
  - plugins/codex/skills/next-chunk.md — new; active chunk detail with full criteria list; handles active:false and has_history:false
  - plugins/codex/skills/plan.md — new; interview-first milestone creation; all inputs collected before any MCP tool call
requires:
  - slice: S01
    provides: milestone_list, milestone_get MCP tools; Milestone, ChunkRef, MilestoneStatus types
  - slice: S02
    provides: cycle_status, cycle_advance, chunk_status MCP tools
  - slice: S03
    provides: milestone_create, spec_create MCP tools
affects: []
key_files:
  - plugins/codex/AGENTS.md
  - plugins/codex/skills/gate-check.md
  - plugins/codex/skills/spec-show.md
  - plugins/codex/skills/cycle-status.md
  - plugins/codex/skills/next-chunk.md
  - plugins/codex/skills/plan.md
key_decisions:
  - "cycle-status and next-chunk kept as separate skills: cycle-status = overview first, next-chunk = chunk detail + criteria list (single-responsibility per skill)"
  - "plan skill strictly interview-first: all 3 input-gathering steps complete before any MCP tool call; milestone_list check is step 4"
  - "AGENTS.md tables restricted to 8 most workflow-relevant MCP tools to stay under 60-line limit (actual: 34 lines)"
  - "Codex skills are flat .md files with YAML frontmatter (name + description), not subdirectory SKILL.md; matches agent-skills flat convention"
patterns_established:
  - "Codex skill files are flat .md files with YAML frontmatter (name + description) matching agent-skills flat-file convention"
  - "Skills calling cycle_status must explicitly handle {\"active\":false} inline with user-facing guidance (suggest /assay:plan)"
  - "Skills calling chunk_status must check has_history before accessing passed/failed gate counts"
observability_surfaces:
  - "wc -l plugins/codex/AGENTS.md — verify line count stays ≤60"
  - "cat plugins/codex/skills/<name>.md — inspect skill content directly"
  - "MCP tool errors surface directly in Codex when a skill fails; skill files handle known edge cases to avoid silent failure"
drill_down_paths:
  - .kata/milestones/M005/slices/S06/tasks/T01-SUMMARY.md
duration: 15min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
---

# S06: Codex Plugin

**Replaced 3-line AGENTS.md stub with a 34-line workflow reference and authored 5 self-contained skill files that give Codex users the full Assay spec-driven development cycle.**

## What Happened

S06 was a pure-content slice with no Rust compilation. All 6 files were authored in a single pass (T01) in dependency order: AGENTS.md first as the anchor reference document, then skills gate-check → spec-show → cycle-status → next-chunk → plan.

`AGENTS.md` (34 lines, well under the 60-line cap) contains: a one-paragraph workflow intro, a skills table listing all 5 commands with descriptions, an MCP tools table with the 8 most workflow-relevant tools, and 5 numbered workflow steps (plan → read → implement → gate-check → advance).

`gate-check.md` and `spec-show.md` are direct ports from `plugins/claude-code/skills/*/SKILL.md` with frontmatter adapted to flat-file format. No semantic changes to the logic.

`cycle-status.md` is a new overview skill. It calls `cycle_status` first; if `{"active":false}` it stops with guidance to run `/assay:plan`. If active, it calls `chunk_status` for the active chunk and renders a compact table with milestone name/phase, chunk progress (X/N), active chunk slug, and latest gate counts. Handles `has_history:false` explicitly.

`next-chunk.md` is a new chunk-detail skill. It calls `cycle_status` first (same active:false handling), then `chunk_status`, then `spec_get` to load the full criteria list. Output is chunk slug/name + gate status + full criteria grouped by executable vs descriptive.

`plan.md` is a new interview-first skill. Steps 1–3 collect milestone goal/name/slug, chunk list, and criteria per chunk through conversation only. Step 4 is the first MCP call (`milestone_list` for slug collision check). Steps 5–6 call `milestone_create` and `spec_create` per chunk. Output includes the mandatory warning that `cmd` fields must be manually added to generated `gates.toml` files before gates are runnable (per D076 known limitation).

`plugins/codex/skills/.gitkeep` was deleted.

## Verification

All 18 slice-level verification checks passed:
- `AGENTS.md` line count: 34 lines (≤60 ✓)
- All 5 skill files exist
- `.gitkeep` removed
- Tool name correctness: `gate_run` in gate-check, `spec_get` in spec-show, `cycle_status`+`chunk_status` in cycle-status, `cycle_status`+`chunk_status`+`spec_get` in next-chunk, `milestone_create`+`spec_create` in plan
- Graceful degradation: `active:false` handling confirmed in cycle-status and next-chunk
- Interview-first ordering confirmed in plan (goal/chunks/criteria before MCP calls)
- `cmd` editing note present in plan
- AGENTS.md mentions all 5 skills

## Requirements Advanced

- R048 — Codex plugin AGENTS.md + 5 skills delivered; workflow guide + gate-check, spec-show, cycle-status, next-chunk, plan skills all authored and verified

## Requirements Validated

- R048 — All deliverables verified by grep assertions and structural inspection; 18/18 checks pass

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None. S06 is a pure-markdown slice — all deliverables authored exactly as planned. The slice plan called for 4 skills; the actual count is 5 (next-chunk was listed in Must-Haves but plan.md was the listed 5th task deliverable — both were completed).

## Known Limitations

- Real Codex runtime execution is UAT-only. Skill files handle the known edge cases (active:false, has_history:false) in their logic, but the actual Codex UX has not been exercised.
- `cmd` fields in generated `gates.toml` files require manual editing before gates are runnable (D076). The plan skill warns about this inline, but it is a real usability gap.

## Follow-ups

- UAT: A developer should install the plugin in Codex (`ln -s .../plugins/codex/skills .agents/skills/assay`) and exercise all 5 skills against a real project with a real milestone.
- The 5th skill `next-chunk` is listed in the plan but the T01 summary says "5 skills" — confirmed: gate-check, spec-show, cycle-status, next-chunk, plan = 5 skills, 6 total files (including AGENTS.md).

## Files Created/Modified

- `plugins/codex/AGENTS.md` — replaced 3-line stub with 34-line workflow reference
- `plugins/codex/skills/gate-check.md` — new: ported from claude-code gate-check
- `plugins/codex/skills/spec-show.md` — new: ported from claude-code spec-show
- `plugins/codex/skills/cycle-status.md` — new: milestone overview skill
- `plugins/codex/skills/next-chunk.md` — new: active chunk detail skill
- `plugins/codex/skills/plan.md` — new: interview-first milestone creation skill
- `plugins/codex/skills/.gitkeep` — deleted

## Forward Intelligence

### What the next slice should know
- The Codex plugin is installed via symlink per README: `ln -s .../plugins/codex/skills .agents/skills/assay`. This is not auto-installed; it requires a manual step during Codex setup.
- Skill files are flat `.md` files (not subdirectory SKILL.md). Future skills for Codex should follow this flat-file convention.
- AGENTS.md is auto-loaded by Codex as agent instructions. It is the single workflow reference; skills are separate invocable commands.

### What's fragile
- The `plan` skill relies on Codex respecting the interview-first ordering. If Codex auto-collapses steps, it may call `milestone_create` before all inputs are collected. The skill text is explicit about this, but it is a prompt-following risk.
- `chunk_status` `has_history` handling: skills check this field before rendering gate counts. If the MCP schema changes this field name, the skills will silently show wrong output.

### Authoritative diagnostics
- `wc -l plugins/codex/AGENTS.md` — line count. Must stay ≤60 as AGENTS.md expands.
- `cat plugins/codex/skills/<name>.md` — primary inspection surface for skill content.
- MCP tool errors surface directly in Codex — no additional logging needed.

### What assumptions changed
- None. S06 was a pure-content slice with no runtime surprises.
