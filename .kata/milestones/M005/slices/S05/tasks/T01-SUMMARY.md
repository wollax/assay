---
id: T01
parent: S05
milestone: M005
provides:
  - plugins/claude-code/skills/plan/SKILL.md — interview-first milestone creation skill
  - plugins/claude-code/skills/status/SKILL.md — cycle status display skill
  - plugins/claude-code/skills/next-chunk/SKILL.md — active chunk context loading skill
  - plugins/claude-code/CLAUDE.md — rewritten with 5-skill table, 11-tool table, workflow paragraph
key_files:
  - plugins/claude-code/skills/plan/SKILL.md
  - plugins/claude-code/skills/status/SKILL.md
  - plugins/claude-code/skills/next-chunk/SKILL.md
  - plugins/claude-code/CLAUDE.md
key_decisions:
  - plan skill uses interview-first pattern — all input collection precedes any MCP tool call to avoid premature tool calls
  - next-chunk handles two null states: active=false (no milestone) and active_chunk_slug=null (all chunks done, in Verify phase)
  - CLAUDE.md kept to 33 lines (well within 50-line budget) by using tables for reference and one paragraph for workflow
patterns_established:
  - skill interview-first pattern: conversational collection → confirm summary → then tool calls
  - skill null-guard pattern: check cycle_status.active before proceeding, check active_chunk_slug for Verify phase
observability_surfaces:
  - none (pure documentation task)
duration: 10min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T01: Write plan/status/next-chunk skill files and update CLAUDE.md

**Three new Claude Code skills and a rewritten CLAUDE.md expose the full M005 development cycle workflow to Claude Code users.**

## What Happened

Created `plan/SKILL.md` with an interview-first pattern: Step 1 explicitly collects goal, chunk count, chunk names/slugs, and criteria per chunk before Step 2 calls `milestone_create`. This ordering is enforced by structure — the interview heading appears on line 6, `milestone_create` on line 24.

Created `status/SKILL.md` as a minimal 2-step skill: call `cycle_status`, branch on `active: false` vs. active milestone to display name/phase/chunk/progress.

Created `next-chunk/SKILL.md` with 3 sequential tool calls plus two null guards: one for `active: false` (no active milestone) and one for `active_chunk_slug: null` (milestone in Verify phase — all chunks done, user should `pr_create`).

Rewrote `CLAUDE.md` to 33 lines: one workflow paragraph, a 5-row Skills table, and an 11-row MCP Tools table. Removed the old 2-command table.

## Verification

```
plugins/claude-code/skills/plan/SKILL.md     ✓ exists, name: plan in frontmatter
plugins/claude-code/skills/status/SKILL.md   ✓ exists, name: status in frontmatter
plugins/claude-code/skills/next-chunk/SKILL.md ✓ exists, name: next-chunk in frontmatter
Interview before tool: line 6 < line 24      ✓ OK
null/pr_create guard count in next-chunk: 1  ✓ OK
CLAUDE.md line count: 33                     ✓ ≤50
/assay: occurrences in CLAUDE.md: 6          ✓ ≥5
cycle_status, pr_create, milestone_create    ✓ all present in CLAUDE.md
```

## Diagnostics

Inspect with: `cat plugins/claude-code/skills/plan/SKILL.md` to verify interview ordering; `wc -l plugins/claude-code/CLAUDE.md` to confirm line budget.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `plugins/claude-code/skills/plan/SKILL.md` — new: interview-first milestone creation skill (5 steps)
- `plugins/claude-code/skills/status/SKILL.md` — new: cycle status display skill (2 steps)
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new: active chunk context loading skill (3 steps + null guards)
- `plugins/claude-code/CLAUDE.md` — rewritten: 33 lines, 5-skill table, 11-tool table, workflow paragraph
