# S06: Codex Plugin — Research

**Date:** 2026-03-20

## Summary

S06 is a pure-markdown slice — no Rust changes. It replaces the 3-line `plugins/codex/AGENTS.md` stub with a concise workflow reference and adds 5 skill files to `plugins/codex/skills/`. All 8 MCP tools the skills consume were fully delivered in S01–S04 and verified working. There is zero risk of compilation errors or test regressions; `just ready` cannot be broken by markdown changes.

The primary design challenge is information density: AGENTS.md must stay under ~60 lines (injected into every Codex conversation), skill files must be self-contained and instruction-complete, and the `plan` skill must enforce an interview-first pattern to avoid calling `milestone_create` on invocation without user input.

S05's new skills (`plan`, `status`/`cycle-status`, `next-chunk`) have not been written yet — the `plugins/claude-code/skills/` directory only contains the original `gate-check` and `spec-show` skills. S06 must design the `cycle-status`, `next-chunk`, and `plan` skills from first principles (not port them from a completed S05 source), using the MCP tool signatures confirmed below as the authoritative contract.

## Recommendation

Write all 6 files in a single T01 pass: `AGENTS.md` first as the anchor/reference table, then the 5 skill files in dependency order (gate-check → spec-show → cycle-status → next-chunk → plan). No scaffolding, no Rust, no tests needed. Verify with grep content checks at the end.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Gate-check skill logic | `plugins/claude-code/skills/gate-check/SKILL.md` | Proven structure: Steps → Output Format; direct port to flat `.md` |
| Spec-show skill logic | `plugins/claude-code/skills/spec-show/SKILL.md` | Same — direct port, change no semantics |
| MCP tool parameters | `crates/assay-mcp/src/server.rs` param structs | Authoritative source for tool names, param field names, and response shapes |

## Existing Code and Patterns

- `plugins/claude-code/skills/gate-check/SKILL.md` — port source; uses `spec_list` + `gate_run`; Output Format section specifies concise pass/fail reporting
- `plugins/claude-code/skills/spec-show/SKILL.md` — port source; uses `spec_list` + `spec_get`; Output Format groups criteria by executable vs descriptive
- `plugins/codex/AGENTS.md` — current 3-line stub; replace entirely with concise tables (skills, CLI, MCP)
- `plugins/claude-code/CLAUDE.md` — style reference for AGENTS.md: 39 lines, three tables, short workflow paragraph
- `crates/assay-mcp/src/server.rs` — MCP tool definitions; all tool names, param structs, and response shapes confirmed below

## MCP Tools Reference for Skills

### Tools consumed by S06 skills

| Tool | Params | Returns | Used by |
|------|--------|---------|---------|
| `spec_list` | (none) | `{ specs: [{name, description, criteria_count, format}], errors }` | gate-check, spec-show |
| `spec_get` | `name: String, resolve?: bool` | Full spec with criteria array | spec-show, cycle-status, next-chunk |
| `gate_run` | `spec: String, include_evidence?: bool` | Gate run results per criterion | gate-check |
| `cycle_status` | (none) | `{ milestone_slug, milestone_name, phase, active_chunk_slug?, completed_count, total_count }` or `{"active":false}` | cycle-status, next-chunk |
| `chunk_status` | `chunk_slug: String` | `{ chunk_slug, has_history, latest_run_id?, passed?, failed?, required_failed? }` | cycle-status, next-chunk |
| `milestone_list` | (none) | `Vec<Milestone>` JSON array | plan (check existing) |
| `milestone_create` | `slug, name, description?, chunks: [{slug, name, criteria: [String]}]` | Created milestone JSON | plan |
| `spec_create` | `slug, name, description?, milestone_slug?, criteria: [String]` | Created spec JSON | plan |

### `cycle_status` response shapes

**Active milestone:** `{"milestone_slug":"auth","milestone_name":"Auth Layer","phase":"InProgress","active_chunk_slug":"login","completed_count":2,"total_count":5}`

**No active milestone:** `{"active":false}`

**`chunk_status` with no history:** `{"chunk_slug":"login","has_history":false}`

**`chunk_status` with results:** `{"chunk_slug":"login","has_history":true,"latest_run_id":"abc123","passed":3,"failed":1,"required_failed":1}`

### `milestone_create` chunks input
Each chunk entry is `{ slug: String, name: String, criteria: Vec<String> }`. Criteria are text descriptions only — no `cmd` field. The generated gates.toml will require manual editing to add executable commands (known limitation, D076).

### Full tool list (for AGENTS.md MCP table — S06-relevant subset)
Pre-existing: `spec_list`, `spec_get`, `spec_validate`, `gate_run`, `gate_evaluate`, `gate_history`, `gate_report`, `session_create`, `session_get`, `session_list`, `session_update`, `worktree_create`, `worktree_list`, `worktree_status`, `worktree_cleanup`, `run_manifest`, `orchestrate_run`, `orchestrate_status`

New in M005: `milestone_list`, `milestone_get`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_create`, `spec_create`, `pr_create`

AGENTS.md should table only the 8 most workflow-relevant tools; the full 30+ list would defeat conciseness.

## Skill File Format

Skills are flat `.md` files (not subdirectories):

```
plugins/codex/skills/
  gate-check.md
  spec-show.md
  cycle-status.md
  next-chunk.md
  plan.md
```

No frontmatter required for Codex skills — the filename is the skill name. The S06-CONTEXT.md says "same frontmatter + `## Steps` + `## Output Format` structure as Claude Code skills" — but the Claude Code skills do use frontmatter (`name:`, `description:`). Include frontmatter for consistency and future portability; it's harmless as a plain comment block for Codex's `.agents/skills/` loader.

Claude Code skill format observed:
```markdown
---
name: gate-check
description: >
  Run quality gates for a spec and report pass/fail results.
  ...
---

# Gate Check

## Steps
1. ...

## Output Format
...
```

Use this exact format for all 5 skills.

## Constraints

- `AGENTS.md` must stay ≤60 lines (target: 40–55). Claude Code's `CLAUDE.md` at 39 lines is the benchmark.
- Skill files are flat (`gate-check.md`), not subdirectories — matches Codex `.agents/skills/` convention (README confirms).
- `plan` skill: interview-first — **never** call `milestone_create` until the user has answered: (1) milestone goal, (2) chunk list with names, (3) criteria per chunk. This is D066/D075 territory.
- `cycle-status` and `next-chunk` are kept separate per user decision in S06-CONTEXT: `cycle-status` = overview first, `next-chunk` = chunk detail first. Slight overlap is intentional.
- No Rust changes. No `just ready` run needed. Only content verification (grep).
- Skills in `plugins/codex/skills/` must remove the `.gitkeep` placeholder.

## Common Pitfalls

- **S05 skills don't exist yet** — Don't reference `plugins/claude-code/skills/plan/SKILL.md` as a port source; it wasn't written in S05. Design cycle-status, next-chunk, and plan from MCP signatures directly.
- **`plan` skill calling tools before interview** — The skill must collect all inputs before calling any MCP tool. Common failure: calling `milestone_list` or `milestone_create` in step 1 before asking the user anything.
- **AGENTS.md line count** — Easy to exceed 60 lines if every tool gets a row. Table only the 8–10 most relevant tools for the dev cycle workflow.
- **`cycle_status` returns `{"active":false}` when no milestone is in_progress** — Skills must handle this gracefully (e.g., suggest running `/assay:plan`), not fail silently.
- **`chunk_status` with `has_history: false`** — When a chunk has never been gate-run, the response has no `passed`/`failed` fields. Skills must not assume these exist.
- **Codex has no native hook system** — Unlike Claude Code, no hooks.json, no Stop hook, no PostToolUse. The skills alone are the integration surface. Don't reference hooks in AGENTS.md.
- **`spec_create` criteria are text-only** — Wizard-generated specs require manual editing to add `cmd` fields before gates are runnable. Mention this in the `plan` skill output format.

## Open Risks

- S05 was not yet merged at research time — the Claude Code `CLAUDE.md` may be updated by S05 and diverge from the current 39-line version used as style reference. S06's AGENTS.md style is based on the current CLAUDE.md.
- If S05 writes `cycle-status` and `next-chunk` skills with different semantics than designed here, there may be minor inconsistency between the two plugins. Not a blocking risk for S06 delivery — the plugins can diverge on details.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Markdown / documentation | none needed | n/a — pure content authoring |

## Sources

- `plugins/claude-code/skills/gate-check/SKILL.md` — gate-check port source (direct inspection)
- `plugins/claude-code/skills/spec-show/SKILL.md` — spec-show port source (direct inspection)
- `plugins/claude-code/CLAUDE.md` — AGENTS.md style reference (39 lines, three tables)
- `crates/assay-mcp/src/server.rs` — MCP tool param structs and response types (authoritative)
- `crates/assay-core/src/milestone/cycle.rs` — CycleStatus struct fields (authoritative)
- `plugins/codex/AGENTS.md` — current stub (to be replaced)
- `plugins/codex/README.md` — skill installation convention (`.agents/skills/assay`)
- `.kata/milestones/M005/slices/S06/S06-CONTEXT.md` — scope decisions and constraints (authoritative for what to build)
