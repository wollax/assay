# S03: OpenCode Plugin with Full Skill Parity

**Goal:** Create `plugins/opencode/AGENTS.md` and 5 skill files matching the Codex plugin exactly, completing three-platform plugin parity.
**Demo:** `plugins/opencode/` contains `AGENTS.md` (≤60 lines) and `skills/gate-check.md`, `skills/spec-show.md`, `skills/cycle-status.md`, `skills/next-chunk.md`, `skills/plan.md`; `.gitkeep` files removed from `skills/` and `agents/`; all MCP tool names verified correct.

## Must-Haves

- `plugins/opencode/AGENTS.md` exists, ≤60 lines, correct skills table and MCP tools table with all 10 tool names
- `plugins/opencode/skills/gate-check.md` exists with correct frontmatter and content
- `plugins/opencode/skills/spec-show.md` exists with correct frontmatter and content
- `plugins/opencode/skills/cycle-status.md` exists with correct frontmatter and content
- `plugins/opencode/skills/next-chunk.md` exists with correct frontmatter and content (both null guards present)
- `plugins/opencode/skills/plan.md` exists with interview-first pattern and cmd-editing warning
- `plugins/opencode/skills/.gitkeep` deleted
- `plugins/opencode/agents/.gitkeep` deleted
- `plugins/opencode/opencode.json` not modified

## Proof Level

- This slice proves: operational (file existence and content structural checks)
- Real runtime required: no (pure markdown, no compilation)
- Human/UAT required: no (structural checks are sufficient per S03-RESEARCH.md)

## Verification

```bash
# File existence checks
test -f plugins/opencode/AGENTS.md
test -f plugins/opencode/skills/gate-check.md
test -f plugins/opencode/skills/spec-show.md
test -f plugins/opencode/skills/cycle-status.md
test -f plugins/opencode/skills/next-chunk.md
test -f plugins/opencode/skills/plan.md

# .gitkeep removed
test ! -f plugins/opencode/skills/.gitkeep
test ! -f plugins/opencode/agents/.gitkeep

# AGENTS.md ≤60 lines
test $(wc -l < plugins/opencode/AGENTS.md) -le 60

# All 10 MCP tool names present in AGENTS.md
grep -q 'spec_list' plugins/opencode/AGENTS.md
grep -q 'spec_get' plugins/opencode/AGENTS.md
grep -q 'gate_run' plugins/opencode/AGENTS.md
grep -q 'cycle_status' plugins/opencode/AGENTS.md
grep -q 'cycle_advance' plugins/opencode/AGENTS.md
grep -q 'chunk_status' plugins/opencode/AGENTS.md
grep -q 'milestone_list' plugins/opencode/AGENTS.md
grep -q 'milestone_create' plugins/opencode/AGENTS.md
grep -q 'spec_create' plugins/opencode/AGENTS.md
grep -q 'pr_create' plugins/opencode/AGENTS.md

# Skill files are flat .md (not subdirectory SKILL.md)
test $(find plugins/opencode/skills -name 'SKILL.md' | wc -l) -eq 0

# Interview-first pattern in plan.md (no tool call before all inputs gathered)
grep -q 'before any MCP tool' plugins/opencode/skills/plan.md

# Two null guards in next-chunk.md
grep -q '"active":false' plugins/opencode/skills/next-chunk.md
grep -q 'active_chunk_slug' plugins/opencode/skills/next-chunk.md

# cmd editing warning in plan.md
grep -q 'cmd' plugins/opencode/skills/plan.md

# opencode.json untouched
test -f plugins/opencode/opencode.json
```

## Observability / Diagnostics

- Runtime signals: none (pure markdown, no runtime boundary)
- Inspection surfaces: `ls plugins/opencode/skills/`, `wc -l plugins/opencode/AGENTS.md`
- Failure visibility: grep checks for required tool names and patterns are self-describing
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `plugins/codex/AGENTS.md`, `plugins/codex/skills/*.md` (verbatim reference)
- New wiring introduced in this slice: none (pure markdown, no code wiring)
- What remains before the milestone is truly usable end-to-end: S04 (gate history analytics), S05 (TUI analytics screen)

## Tasks

- [x] **T01: Author AGENTS.md and all 5 skill files for the OpenCode plugin** `est:30m`
  - Why: Delivers the complete S03 deliverable — all 7 files needed for OpenCode plugin parity, plus .gitkeep removal
  - Files: `plugins/opencode/AGENTS.md`, `plugins/opencode/skills/gate-check.md`, `plugins/opencode/skills/spec-show.md`, `plugins/opencode/skills/cycle-status.md`, `plugins/opencode/skills/next-chunk.md`, `plugins/opencode/skills/plan.md`
  - Do: (1) Delete `plugins/opencode/skills/.gitkeep` and `plugins/opencode/agents/.gitkeep`. (2) Write `plugins/opencode/AGENTS.md` — identical to `plugins/codex/AGENTS.md` with "Codex" replaced by "OpenCode" in the title; include pr_create in MCP tools table (10 tools total). (3) Copy all 5 skill files from `plugins/codex/skills/` to `plugins/opencode/skills/` unchanged — content is already platform-neutral. (4) Verify constraints: AGENTS.md ≤60 lines; flat .md files only; both null guards present in next-chunk.md; interview-first in plan.md; cmd-editing warning in plan.md.
  - Verify: Run the verification bash block from the Verification section above
  - Done when: All verification checks pass with exit 0

## Files Likely Touched

- `plugins/opencode/AGENTS.md` (new)
- `plugins/opencode/skills/gate-check.md` (new)
- `plugins/opencode/skills/spec-show.md` (new)
- `plugins/opencode/skills/cycle-status.md` (new)
- `plugins/opencode/skills/next-chunk.md` (new)
- `plugins/opencode/skills/plan.md` (new)
- `plugins/opencode/skills/.gitkeep` (deleted)
- `plugins/opencode/agents/.gitkeep` (deleted)
