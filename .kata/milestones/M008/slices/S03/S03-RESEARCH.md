# S03: OpenCode Plugin with Full Skill Parity — Research

**Date:** 2026-03-24

## Summary

S03 is a pure-markdown slice with no Rust changes, no compilation, and no schema changes. The goal is to create `plugins/opencode/AGENTS.md` and 5 skill files (`gate-check.md`, `spec-show.md`, `cycle-status.md`, `next-chunk.md`, `plan.md`) in `plugins/opencode/skills/`, matching the Codex plugin structure exactly. All conventions are locked by prior decisions (D082, D084, D085, D119). The Codex plugin (`plugins/codex/`) is the authoritative reference template.

The OpenCode plugin scaffold already exists with `package.json`, `opencode.json`, `tsconfig.json`, and placeholder `.gitkeep` files in `skills/`, `agents/`, `commands/`, `plugins/`, and `tools/` directories. The `.gitkeep` files in `skills/` and `agents/` must be removed (replaced by real content).

Implementation is a single-task pass: copy Codex skill content verbatim, adjusting any Codex-specific wording if needed (there is none — skills are tool-name-referenced, not harness-specific). AGENTS.md follows the Codex AGENTS.md line-for-line.

## Recommendation

Directly mirror the Codex plugin. Copy all 5 skill files from `plugins/codex/skills/` to `plugins/opencode/skills/` unchanged — the content is already platform-neutral (references MCP tool names, not Codex-specific invocation). Create `plugins/opencode/AGENTS.md` identical to `plugins/codex/AGENTS.md` (skill table + MCP tools table + workflow steps). Remove `.gitkeep` from `skills/` and `agents/`.

No modifications to `opencode.json`, `package.json`, or `tsconfig.json` — scaffold is left as-is.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Skill content for all 5 skills | `plugins/codex/skills/*.md` — already authored and verified (M005/S06) | Content is platform-neutral; references only MCP tool names. Direct copy avoids drift. |
| AGENTS.md structure and line count | `plugins/codex/AGENTS.md` — 34 lines, ≤60 cap, verified structural checks | Same tables and workflow steps apply identically to OpenCode |
| MCP tool name list | `plugins/codex/AGENTS.md` MCP tools table | Locked by D067; all 10 tool names confirmed in M005 |

## Existing Code and Patterns

- `plugins/codex/AGENTS.md` — authoritative AGENTS.md template; copy verbatim replacing "Codex" with "OpenCode" in the title
- `plugins/codex/skills/gate-check.md` — copy to `plugins/opencode/skills/gate-check.md` unchanged
- `plugins/codex/skills/spec-show.md` — copy to `plugins/opencode/skills/spec-show.md` unchanged
- `plugins/codex/skills/cycle-status.md` — copy to `plugins/opencode/skills/cycle-status.md` unchanged
- `plugins/codex/skills/next-chunk.md` — copy to `plugins/opencode/skills/next-chunk.md` unchanged
- `plugins/codex/skills/plan.md` — copy to `plugins/opencode/skills/plan.md` unchanged
- `plugins/opencode/opencode.json` — do not modify; current content is a minimal plugin descriptor (no skills registration needed)
- `plugins/opencode/skills/.gitkeep` — delete; replaced by 5 skill files
- `plugins/opencode/agents/.gitkeep` — delete; AGENTS.md goes at `plugins/opencode/AGENTS.md` (top level, same as Codex)

## Constraints

- Skill files must be flat `.md` files in `plugins/opencode/skills/` — NOT subdirectory SKILL.md format (D119)
- AGENTS.md ≤60 lines (Codex/M005 convention, D082)
- All MCP tool names must match exactly: `spec_list`, `spec_get`, `gate_run`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_list`, `milestone_create`, `spec_create`, `pr_create` (D067)
- Interview-first pattern in `plan.md` — all inputs collected before any MCP tool call (D084)
- Two null guards in `next-chunk.md`: `active: false` → no active milestone message; `active_chunk_slug: null` → all chunks complete, run `assay pr create` (D085)
- `cmd` editing warning in `plan.md` output section (D076)
- No Rust changes, no schema changes, no `just ready` run needed beyond file existence check

## Common Pitfalls

- **Wrong AGENTS.md location** — AGENTS.md goes at `plugins/opencode/AGENTS.md` (top level), not inside `agents/` subdirectory. The `agents/` directory holds agent config files in Claude Code; for OpenCode/Codex, AGENTS.md is at the plugin root.
- **Forgetting to remove .gitkeep** — `plugins/opencode/skills/.gitkeep` and `plugins/opencode/agents/.gitkeep` must be deleted when adding real content; leaving them causes `git status` noise and the structural check to count extra files.
- **Subdirectory SKILL.md format** — Claude Code uses `skills/<name>/SKILL.md` with frontmatter. Codex/OpenCode use flat `skills/<name>.md`. Do not add subdirectories.
- **Platform-specific wording** — Codex skills have no Codex-specific invocation syntax. Content references MCP tool names only, so it's already platform-neutral. No text substitutions needed in skill content beyond AGENTS.md title.
- **Modifying opencode.json** — The opencode.json is a package descriptor, not a skills registry. Skills in OpenCode are loaded by file convention, not JSON registration. Leave it as-is.

## Open Risks

- None. S03 is pure markdown with all conventions locked by D082, D084, D085, D119. The Codex plugin (M005/S06) already proved the content and verified MCP tool names. No compilation, no testing, no schema changes.

## Verification Checklist (Boundary Map deliverables)

From the M008 Boundary Map, S03 must produce:
- [x] `plugins/opencode/AGENTS.md` — workflow guide with skills/MCP tables
- [x] `plugins/opencode/skills/gate-check.md`
- [x] `plugins/opencode/skills/spec-show.md`
- [x] `plugins/opencode/skills/cycle-status.md`
- [x] `plugins/opencode/skills/next-chunk.md`
- [x] `plugins/opencode/skills/plan.md`
- [x] `.gitkeep` files removed from `skills/` and `agents/` directories

Success criteria from M008-ROADMAP.md:
> OpenCode plugin installed in `plugins/opencode/` with AGENTS.md + 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan) that reference correct MCP tool names

Proof strategy: file existence and structural checks (no compilation or integration tests needed).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| OpenCode plugin | none | none found — not needed; pure markdown |

## Sources

- Codex plugin structure verified by reading `plugins/codex/` (5 skill files + AGENTS.md)
- OpenCode scaffold inspected at `plugins/opencode/` (package.json, opencode.json, .gitkeep placeholders)
- Decisions D082, D084, D085, D119 reviewed from DECISIONS.md
- M008 Boundary Map and S03-CONTEXT.md reviewed
