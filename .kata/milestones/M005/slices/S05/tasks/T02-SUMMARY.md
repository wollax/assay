---
id: T02
parent: S05
milestone: M005
provides:
  - plugins/claude-code/skills/plan/SKILL.md — interview-first milestone+spec creation skill
  - plugins/claude-code/skills/status/SKILL.md — cycle status display skill
  - plugins/claude-code/skills/next-chunk/SKILL.md — active chunk context loader skill
  - plugins/claude-code/CLAUDE.md — updated with full workflow: skills + CLI + MCP tables
key_files:
  - plugins/claude-code/skills/plan/SKILL.md
  - plugins/claude-code/skills/status/SKILL.md
  - plugins/claude-code/skills/next-chunk/SKILL.md
  - plugins/claude-code/CLAUDE.md
key_decisions:
  - none — pure markdown content following established skill format (frontmatter + Steps + Output Format)
patterns_established:
  - Skill interview-first pattern: /assay:plan always collects all inputs conversationally before any MCP calls
  - Sentinel handling: skills check `"active": false` and null `active_chunk_slug` before proceeding
observability_surfaces:
  - Skill files are not executable; failure shows as "skill not found" in Claude Code
duration: ~15m
verification_result: passed
completed_at: 2026-03-20
blocker_discovered: false
---

# T02: Write skill files and update CLAUDE.md

**Created three new Claude Code skills (`/assay:plan`, `/assay:status`, `/assay:next-chunk`) and updated CLAUDE.md with a full workflow reference covering all 5 skills, 5 CLI commands, and all 11 MCP tools (39 lines, well under the 60-line limit).**

## What Happened

All four deliverables are pure markdown. The existing `gate-check/SKILL.md` and `spec-show/SKILL.md` were used as format references — frontmatter with `name:` + `description:`, then `## Steps` and `## Output Format` sections.

**plan/SKILL.md:** Interview-first workflow — Step 1 explicitly collects milestone goal, chunk count, and per-chunk slug/name/criteria before any MCP calls. Step 2 calls `milestone_create`, Step 3 calls `spec_create` once per chunk with `milestone_slug` and criteria array. Step 4 confirms and warns that generated gates have no `cmd` field.

**status/SKILL.md:** Calls `cycle_status` (no params), handles `{"active": false}` sentinel with a "no active milestone" message, otherwise renders milestone slug/name, phase, active chunk slug, and a `[x][ ][ ]` progress display derived from `completed_count`/`total_count`.

**next-chunk/SKILL.md:** Calls `cycle_status` → extracts `active_chunk_slug` → handles null/missing case → calls `chunk_status` for pass/fail → calls `spec_get` for full criteria → presents criteria list with ✓/✗ status from chunk_status.

**CLAUDE.md:** Replaced with a concise three-table format: Skills, CLI Commands, MCP Tools. 39 lines total.

## Verification

All grep checks pass:
```
grep -l "milestone_create" plugins/claude-code/skills/plan/SKILL.md  → exits 0
grep "spec_create" plugins/claude-code/skills/plan/SKILL.md           → exits 0
grep "cycle_status" plugins/claude-code/skills/status/SKILL.md        → exits 0
grep "chunk_status" plugins/claude-code/skills/next-chunk/SKILL.md    → exits 0
grep "spec_get" plugins/claude-code/skills/next-chunk/SKILL.md        → exits 0
grep "assay:plan" plugins/claude-code/CLAUDE.md                       → exits 0
grep "pr_create" plugins/claude-code/CLAUDE.md                        → exits 0
wc -l plugins/claude-code/CLAUDE.md                                   → 39 (≤60 ✓)
```

Slice-level checks applicable to this task:
- Content checks all pass (grep above)
- `cargo test --workspace`, `just ready`, bash syntax checks — not in scope for T02 (pure markdown, no Rust changes)

## Diagnostics

Skill files are not executable. Inspection: `cat plugins/claude-code/skills/<name>/SKILL.md`. Failure state is "skill not found" in Claude Code if file is missing or frontmatter `name:` doesn't match the skill command name.

## Deviations

None — followed task plan exactly.

## Known Issues

None.

## Files Created/Modified

- `plugins/claude-code/skills/plan/SKILL.md` — new; interview-first workflow calling `milestone_create` + `spec_create`
- `plugins/claude-code/skills/status/SKILL.md` — new; `cycle_status` display with `{"active":false}` handling
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new; `cycle_status` + `chunk_status` + `spec_get` context loader
- `plugins/claude-code/CLAUDE.md` — replaced; 39-line reference with Skills, CLI, and MCP tables
