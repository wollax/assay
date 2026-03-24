---
estimated_steps: 5
estimated_files: 8
---

# T01: Author AGENTS.md and all 5 skill files for the OpenCode plugin

**Slice:** S03 — OpenCode Plugin with Full Skill Parity
**Milestone:** M008

## Description

Create the complete OpenCode plugin content: `plugins/opencode/AGENTS.md` and 5 skill files (`gate-check.md`, `spec-show.md`, `cycle-status.md`, `next-chunk.md`, `plan.md`) in `plugins/opencode/skills/`. Remove `.gitkeep` placeholder files from `skills/` and `agents/`. All content is copied verbatim from the Codex plugin (`plugins/codex/`), which is platform-neutral (references only MCP tool names). AGENTS.md title changes "Codex" to "OpenCode"; skill content is unchanged.

## Steps

1. Delete `plugins/opencode/skills/.gitkeep` and `plugins/opencode/agents/.gitkeep`
2. Write `plugins/opencode/AGENTS.md` — identical to `plugins/codex/AGENTS.md` with "Codex" replaced by "OpenCode" in the first heading; ensure `pr_create` is included in the MCP tools table (10 tools total)
3. Copy `plugins/codex/skills/gate-check.md` → `plugins/opencode/skills/gate-check.md` (unchanged)
4. Copy `plugins/codex/skills/spec-show.md` → `plugins/opencode/skills/spec-show.md` (unchanged)
5. Copy `plugins/codex/skills/cycle-status.md` → `plugins/opencode/skills/cycle-status.md`, `plugins/codex/skills/next-chunk.md` → `plugins/opencode/skills/next-chunk.md`, and `plugins/codex/skills/plan.md` → `plugins/opencode/skills/plan.md` (all unchanged)

## Must-Haves

- [ ] `plugins/opencode/AGENTS.md` exists, ≤60 lines, contains all 10 MCP tool names (spec_list, spec_get, gate_run, cycle_status, cycle_advance, chunk_status, milestone_list, milestone_create, spec_create, pr_create)
- [ ] `plugins/opencode/skills/gate-check.md` exists with valid frontmatter (name: gate-check)
- [ ] `plugins/opencode/skills/spec-show.md` exists with valid frontmatter (name: spec-show)
- [ ] `plugins/opencode/skills/cycle-status.md` exists with valid frontmatter (name: cycle-status); `{"active":false}` guard present
- [ ] `plugins/opencode/skills/next-chunk.md` exists with both null guards (`{"active":false}` and `active_chunk_slug` null case)
- [ ] `plugins/opencode/skills/plan.md` exists with interview-first pattern ("before any MCP tool") and cmd-editing warning
- [ ] `plugins/opencode/skills/.gitkeep` deleted
- [ ] `plugins/opencode/agents/.gitkeep` deleted
- [ ] `plugins/opencode/opencode.json` not modified (verify content unchanged after task)

## Verification

```bash
# File existence
test -f plugins/opencode/AGENTS.md
test -f plugins/opencode/skills/gate-check.md
test -f plugins/opencode/skills/spec-show.md
test -f plugins/opencode/skills/cycle-status.md
test -f plugins/opencode/skills/next-chunk.md
test -f plugins/opencode/skills/plan.md

# .gitkeep removed
test ! -f plugins/opencode/skills/.gitkeep
test ! -f plugins/opencode/agents/.gitkeep

# AGENTS.md line count
test $(wc -l < plugins/opencode/AGENTS.md) -le 60

# All 10 MCP tool names present
for tool in spec_list spec_get gate_run cycle_status cycle_advance chunk_status milestone_list milestone_create spec_create pr_create; do
  grep -q "$tool" plugins/opencode/AGENTS.md || (echo "MISSING: $tool" && exit 1)
done

# Skill files are flat .md (no subdirectory SKILL.md format)
test $(find plugins/opencode/skills -name 'SKILL.md' | wc -l) -eq 0

# Interview-first pattern in plan.md
grep -q 'before any MCP tool' plugins/opencode/skills/plan.md

# Two null guards in next-chunk.md
grep -q '"active":false' plugins/opencode/skills/next-chunk.md
grep -q 'active_chunk_slug' plugins/opencode/skills/next-chunk.md

# cmd editing warning in plan.md
grep -q 'cmd' plugins/opencode/skills/plan.md

# opencode.json still present and unmodified
test -f plugins/opencode/opencode.json
```

All checks exit 0.

## Observability Impact

- Signals added/changed: None (pure markdown, no runtime boundary)
- How a future agent inspects this: `ls plugins/opencode/skills/` and `wc -l plugins/opencode/AGENTS.md`
- Failure state exposed: grep checks for required patterns are self-describing; any missing tool name or pattern is immediately visible

## Inputs

- `plugins/codex/AGENTS.md` — authoritative template; copy verbatim, change "Codex" to "OpenCode" in title
- `plugins/codex/skills/gate-check.md` — copy unchanged
- `plugins/codex/skills/spec-show.md` — copy unchanged
- `plugins/codex/skills/cycle-status.md` — copy unchanged
- `plugins/codex/skills/next-chunk.md` — copy unchanged
- `plugins/codex/skills/plan.md` — copy unchanged
- S03-RESEARCH.md decisions: D082 (≤60 lines AGENTS.md), D084 (interview-first), D085 (two null guards in next-chunk), D119 (flat .md files, not subdirectory SKILL.md)

## Expected Output

- `plugins/opencode/AGENTS.md` — workflow guide with skills table (5 skills) and MCP tools table (10 tools), ≤60 lines
- `plugins/opencode/skills/gate-check.md` — gate check skill
- `plugins/opencode/skills/spec-show.md` — spec display skill
- `plugins/opencode/skills/cycle-status.md` — cycle overview skill
- `plugins/opencode/skills/next-chunk.md` — chunk detail skill with two null guards
- `plugins/opencode/skills/plan.md` — milestone creation skill with interview-first pattern and cmd-editing warning
- `plugins/opencode/skills/.gitkeep` — deleted
- `plugins/opencode/agents/.gitkeep` — deleted
