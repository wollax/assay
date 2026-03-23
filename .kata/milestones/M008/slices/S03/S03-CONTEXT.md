---
id: S03
milestone: M008
status: ready
---

# S03: OpenCode plugin with full skill parity — Context

## Goal

Deliver a complete OpenCode plugin with AGENTS.md and 5 skills matching the Codex plugin exactly, closing the three-platform parity gap.

## Why this Slice

OpenCode is the third major agent platform. Without a plugin, Assay users on OpenCode have no guided workflow. S03 has no dependencies on other M008 slices — it can ship in parallel. Completing it early removes R057 from the active list.

## Scope

### In Scope

- `plugins/opencode/AGENTS.md` — workflow guide with skills table and MCP tools table, matching Codex AGENTS.md structure and line count (~34 lines, ≤60 cap)
- 5 skill files in `plugins/opencode/skills/`:
  - `gate-check.md` — run quality gates and report pass/fail
  - `spec-show.md` — display a spec's criteria
  - `cycle-status.md` — overview of active milestone and chunk progress
  - `next-chunk.md` — detail view of active chunk with criteria and gate status
  - `plan.md` — interview-guided milestone and spec creation
- Remove `.gitkeep` from `plugins/opencode/skills/` and `plugins/opencode/agents/` (replaced by real files)
- All skill files reference correct MCP tool names from M005 (`spec_list`, `spec_get`, `gate_run`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_list`, `milestone_create`, `spec_create`)
- Skills follow the Codex flat `.md` file convention (D119), not Claude Code's subdirectory SKILL.md format
- Interview-first pattern (D084) in the plan skill
- Two null guards in next-chunk (D085): `active: false` and `active_chunk_slug: null`
- `cmd` editing warning in plan skill output (D076)

### Out of Scope

- Modifying `opencode.json`, `package.json`, or `tsconfig.json` — leave scaffold as-is
- TypeScript implementation — skills are pure markdown
- Hooks (stop hook, post-tool-use) — OpenCode hook format is unknown; defer
- Claude Code SKILL.md subdirectory format — use flat files per D119
- Any Rust code changes — S03 is pure markdown

## Constraints

- Skill content must reference the exact MCP tool names established in M005 (D067)
- Flat `.md` file convention, not subdirectory SKILL.md (D119, consistent with D082)
- AGENTS.md ≤60 lines (Codex convention from M005/S06)
- All files authored in a single task pass (D082 pattern — no compilation, no schema changes, fits one context window)

## Integration Points

### Consumes

- MCP tool names from M005: `spec_list`, `spec_get`, `gate_run`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_list`, `milestone_create`, `spec_create`, `pr_create`
- Codex plugin structure as reference template (`plugins/codex/AGENTS.md` + `plugins/codex/skills/*.md`)

### Produces

- `plugins/opencode/AGENTS.md` — workflow guide for OpenCode agents
- `plugins/opencode/skills/gate-check.md`
- `plugins/opencode/skills/spec-show.md`
- `plugins/opencode/skills/cycle-status.md`
- `plugins/opencode/skills/next-chunk.md`
- `plugins/opencode/skills/plan.md`

## Open Questions

- None — this is a straightforward parity slice with all conventions locked by prior decisions.
