---
id: S06
milestone: M005
status: ready
---

# S06: Codex Plugin — Context

## Goal

Deliver a functional Codex plugin: a concise `AGENTS.md` workflow reference and 5 skills (`gate-check`, `spec-show`, `cycle-status`, `next-chunk`, `plan`) that give Codex users the same spec-driven development cycle experience as Claude Code users.

## Why this Slice

S01–S04 delivered all 8 MCP tools the plugins consume. S05 proved the skill design patterns on Claude Code. S06 now ports and extends those patterns to Codex — the last deliverable needed for M005's milestone definition of done. Nothing depends on S06, so it can proceed immediately.

## Scope

### In Scope

- `plugins/codex/AGENTS.md` — replaced with a concise quick-reference (~40–60 lines): skills table, CLI commands table, MCP tools table; same style as the updated `plugins/claude-code/CLAUDE.md`
- `plugins/codex/skills/gate-check.md` — ported from `claude-code/skills/gate-check/SKILL.md`; same frontmatter + Steps + Output Format structure, flat file (not subdirectory)
- `plugins/codex/skills/spec-show.md` — ported from `claude-code/skills/spec-show/SKILL.md`; same format
- `plugins/codex/skills/cycle-status.md` — new; full chain: `cycle_status` → `chunk_status` → `spec_get`; shows milestone/phase/progress AND active chunk criteria in one invocation (mirrors `next-chunk` depth)
- `plugins/codex/skills/next-chunk.md` — new; dedicated skill that loads the active chunk context (same chain as `cycle-status` but scoped to chunk detail); kept separate for clarity even though `cycle-status` also does the full chain
- `plugins/codex/skills/plan.md` — new; interview-first pattern identical to `claude-code/skills/plan/SKILL.md`: always collects milestone goal, chunk breakdown, and criteria conversationally before calling `milestone_create` + `spec_create`

### Out of Scope

- Codex hook scripts (Stop hook, PostToolUse) — Codex uses AGENTS.md and skills, not hooks.json; no hook infrastructure needed
- `plugins/codex/skills/status.md` as a separate file — `cycle-status.md` covers this surface; a redundant status-only skill is not needed
- Any Rust changes — S06 is pure markdown; all MCP tools it relies on were delivered in S01–S04
- README.md updates — the existing stub is sufficient; plugin installation docs are out of scope
- OpenCode plugin — M008

## Constraints

- Skill files must use the same frontmatter + `## Steps` + `## Output Format` structure as Claude Code skills — consistent format across both plugins
- Skill files are flat (`skills/gate-check.md`), not subdirectories (`skills/gate-check/SKILL.md`) — matches Codex's `.agents/skills/` convention and the roadmap naming
- AGENTS.md must stay concise — it is injected into every Codex conversation; the Claude Code CLAUDE.md at 39 lines is the target; do not exceed ~60 lines
- `plan` skill must be interview-first — never call `milestone_create` on invocation without first collecting inputs conversationally (same constraint as D066/S05)
- `cycle-status` and `next-chunk` are kept as separate skills even though both do the full chain — the user confirmed both; `cycle-status` emphasises progress overview, `next-chunk` emphasises active chunk detail
- No `just ready` impact — pure markdown; verify with `bash -n` not applicable; verify with grep content checks only

## Integration Points

### Consumes

- `milestone_list` MCP tool (S01) — `cycle-status` and `plan` may surface existing milestones
- `milestone_create` MCP tool (S03) — `plan` skill calls this after interview
- `spec_create` MCP tool (S03) — `plan` skill calls this per chunk after interview
- `cycle_status` MCP tool (S02) — `cycle-status` and `next-chunk` call this first
- `chunk_status` MCP tool (S02) — `cycle-status` and `next-chunk` call this for gate pass/fail
- `spec_get` MCP tool (pre-existing) — `cycle-status`, `next-chunk`, and `spec-show` call this for full criteria
- `spec_list` MCP tool (pre-existing) — `gate-check` and `spec-show` use this when no spec argument given
- `gate_run` MCP tool (pre-existing) — `gate-check` calls this
- `plugins/claude-code/skills/gate-check/SKILL.md` — port source for gate-check
- `plugins/claude-code/skills/spec-show/SKILL.md` — port source for spec-show
- `plugins/claude-code/skills/plan/SKILL.md` (S05) — port source for plan (interview-first pattern)
- `plugins/claude-code/skills/next-chunk/SKILL.md` (S05) — port source for next-chunk
- `plugins/claude-code/skills/status/SKILL.md` (S05) — port source for cycle-status

### Produces

- `plugins/codex/AGENTS.md` — concise workflow reference replacing the 3-line stub
- `plugins/codex/skills/gate-check.md` — gate-check skill (ported)
- `plugins/codex/skills/spec-show.md` — spec-show skill (ported)
- `plugins/codex/skills/cycle-status.md` — full-chain status + active chunk context skill (new)
- `plugins/codex/skills/next-chunk.md` — active chunk detail skill (new)
- `plugins/codex/skills/plan.md` — interview-first milestone authoring skill (new)

## Open Questions

- None — all UX and scope decisions confirmed in discuss phase.
