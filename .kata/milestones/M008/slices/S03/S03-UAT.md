# S03: OpenCode Plugin with Full Skill Parity — UAT

**Milestone:** M008
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S03 is pure markdown — no compilation, no runtime, no agent spawning. File existence and content structural checks are the complete verification surface. There is no live-runtime behavior to test.

## Preconditions

- Git repository with `plugins/opencode/` directory present
- `plugins/codex/` exists as reference (for content comparison if desired)

## Smoke Test

```bash
ls plugins/opencode/AGENTS.md plugins/opencode/skills/*.md
# Expected: 6 files listed (AGENTS.md + 5 skills)
```

## Test Cases

### 1. AGENTS.md contains all 10 MCP tools

1. Open `plugins/opencode/AGENTS.md`
2. Check the MCP tools table
3. **Expected:** All 10 tool names present: spec_list, spec_get, gate_run, cycle_status, cycle_advance, chunk_status, milestone_list, milestone_create, spec_create, pr_create

### 2. AGENTS.md line count within limit

1. Run `wc -l plugins/opencode/AGENTS.md`
2. **Expected:** ≤60 lines

### 3. Skills match Codex plugin exactly

1. Run `diff plugins/codex/skills/ plugins/opencode/skills/`
2. **Expected:** No differences (all 5 skill files identical)

### 4. .gitkeep files removed

1. Run `ls plugins/opencode/skills/.gitkeep plugins/opencode/agents/.gitkeep`
2. **Expected:** Both files do not exist (ls returns error)

### 5. opencode.json untouched

1. Run `git diff plugins/opencode/opencode.json`
2. **Expected:** No changes

## Edge Cases

### AGENTS.md title differs from Codex

1. Compare first heading line of `plugins/opencode/AGENTS.md` vs `plugins/codex/AGENTS.md`
2. **Expected:** OpenCode says "OpenCode" where Codex says "Codex"; all other content identical

### Flat file format (no SKILL.md subdirectories)

1. Run `find plugins/opencode/skills -name 'SKILL.md'`
2. **Expected:** No results (skills are flat .md files, not subdirectory SKILL.md)

## Failure Signals

- Missing skill files in `plugins/opencode/skills/`
- MCP tool names missing from AGENTS.md
- `plan.md` missing "before any MCP tool" interview-first pattern
- `next-chunk.md` missing either null guard (`"active":false` or `active_chunk_slug`)
- `.gitkeep` files still present

## Requirements Proved By This UAT

- R057 — OpenCode plugin has AGENTS.md + 5 skills with correct MCP tool references, completing three-platform plugin parity

## Not Proven By This UAT

- Actual OpenCode runtime integration (whether OpenCode can load and use these skills) — this is outside Assay's scope
- MCP tool name correctness at runtime (structural checks only verify string presence, not that the tools exist in the MCP server)

## Notes for Tester

- This is the simplest UAT in M008 — all checks are `grep`, `diff`, `ls`, and `wc`. No compilation or runtime needed.
- The Codex plugin (`plugins/codex/`) is the authoritative reference. A clean `diff` between the two skill directories confirms parity.
