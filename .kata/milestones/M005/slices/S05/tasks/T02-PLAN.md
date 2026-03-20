---
estimated_steps: 4
estimated_files: 4
---

# T02: Write skill files and update CLAUDE.md

**Slice:** S05 — Claude Code Plugin Upgrade
**Milestone:** M005

## Description

Create three new skill files that expose the milestone-driven workflow inside Claude Code: `/assay:plan` (guided milestone authoring), `/assay:status` (cycle progress view), and `/assay:next-chunk` (active chunk context loader). Update CLAUDE.md to document the full workflow — it is injected into every Claude Code conversation and must remain concise while covering all new surfaces.

All deliverables are pure markdown — no Rust, no tests required. Verification is content-grep based.

## Steps

1. Create `plugins/claude-code/skills/plan/SKILL.md` with frontmatter `name: plan` and a Steps section:
   - Step 1 (Interview): Ask the user for the milestone goal/name (derive a slug via slugify), how many chunks (1–7), and per chunk: a name, slug, and a few success criteria (plain descriptions). Do this conversationally before any MCP calls — never call `milestone_create` immediately on invocation.
   - Step 2 (Create milestone): Call `milestone_create` with `{slug, name, description?, chunks: [{slug, name}, ...]}`.
   - Step 3 (Create specs): Call `spec_create` once per chunk with `{slug, name, milestone_slug, criteria: [...]}`.
   - Step 4 (Confirm): Show a summary of created files and remind the user that generated gates have no `cmd` field — runnable commands must be added manually to each `gates.toml`.

2. Create `plugins/claude-code/skills/status/SKILL.md` with frontmatter `name: status` and Steps:
   - Step 1: Call `cycle_status` MCP tool (no params).
   - Step 2: If response contains `"active":false`, report "No active milestone. Run `/assay:plan` to start one."
   - Step 3: Otherwise display: milestone slug + name, current phase, active chunk slug (or "all chunks complete"), progress as `[x][x][ ]` style count (completed/total), and suggest next action (`/assay:next-chunk` to load context, `assay milestone advance` to evaluate gates).

3. Create `plugins/claude-code/skills/next-chunk/SKILL.md` with frontmatter `name: next-chunk` and Steps:
   - Step 1: Call `cycle_status` to find the active milestone slug and `active_chunk_slug`.
   - Step 2: If no active chunk (null `active_chunk_slug` or `active == false`), report "No active chunk — all chunks complete. Use `assay pr create <milestone>` or `assay milestone advance` to finish the milestone."
   - Step 3: Call `chunk_status` with `{chunk_slug: active_chunk_slug}` for pass/fail summary.
   - Step 4: Call `spec_get` with `{name: active_chunk_slug}` to load full criteria.
   - Step 5: Present: chunk slug, each criterion with name + description + pass/fail status (from chunk_status), and suggested next action — fix failing criteria then run `/assay:gate-check <slug>`.

4. Replace `plugins/claude-code/CLAUDE.md` with updated content containing:
   - Short intro: "This project uses Assay for milestone-driven spec development. Use the skills and commands below to work through the development cycle."
   - **Skills** table: `/assay:plan`, `/assay:status`, `/assay:next-chunk`, `/assay:spec-show [name]`, `/assay:gate-check [name]`
   - **CLI Commands** table: `assay plan`, `assay milestone list`, `assay milestone status`, `assay milestone advance`, `assay pr create <slug>`
   - **MCP Tools** table: all 8 new tools (`milestone_list`, `milestone_get`, `milestone_create`, `spec_create`, `cycle_status`, `cycle_advance`, `chunk_status`, `pr_create`) plus existing `spec_list`, `spec_get`, `gate_run`
   - Keep total length under ~60 lines — this is injected into every conversation context.

## Must-Haves

- [ ] `plugins/claude-code/skills/plan/SKILL.md` exists; contains `milestone_create` and `spec_create`; interview step precedes MCP calls; warns about no-cmd limitation
- [ ] `plugins/claude-code/skills/status/SKILL.md` exists; calls `cycle_status`; handles `{"active":false}` case
- [ ] `plugins/claude-code/skills/next-chunk/SKILL.md` exists; calls `cycle_status` + `chunk_status` + `spec_get`; handles null `active_chunk_slug`
- [ ] `plugins/claude-code/CLAUDE.md` references `/assay:plan`, `/assay:status`, `/assay:next-chunk`, and all 8 new MCP tools
- [ ] CLAUDE.md remains concise (≤60 lines)

## Verification

- `grep -l "milestone_create" plugins/claude-code/skills/plan/SKILL.md` — exits 0
- `grep "spec_create" plugins/claude-code/skills/plan/SKILL.md` — exits 0
- `grep "cycle_status" plugins/claude-code/skills/status/SKILL.md` — exits 0
- `grep "chunk_status" plugins/claude-code/skills/next-chunk/SKILL.md` — exits 0
- `grep "spec_get" plugins/claude-code/skills/next-chunk/SKILL.md` — exits 0
- `grep "assay:plan" plugins/claude-code/CLAUDE.md` — exits 0
- `grep "pr_create" plugins/claude-code/CLAUDE.md` — exits 0
- `wc -l plugins/claude-code/CLAUDE.md` — ≤60 lines

## Observability Impact

- Signals added/changed: None (pure markdown content)
- How a future agent inspects this: `cat plugins/claude-code/skills/<name>/SKILL.md` — inspect skill instructions; `cat plugins/claude-code/CLAUDE.md` — inspect injected context
- Failure state exposed: None — skill files are not executable; failure shows as "skill not found" in Claude Code

## Inputs

- `plugins/claude-code/skills/gate-check/SKILL.md` — reference format: frontmatter + `## Steps` + `## Output Format`
- `plugins/claude-code/skills/spec-show/SKILL.md` — reference for `$ARGUMENTS` convention and output format
- `plugins/claude-code/CLAUDE.md` — existing content to replace
- S05-RESEARCH.md MCP Tool Reference table — authoritative tool names and params for skills
- S05-RESEARCH.md Common Pitfalls — `{"active":false}` sentinel shape; `active_chunk_slug` can be null

## Expected Output

- `plugins/claude-code/skills/plan/SKILL.md` — new skill with interview-first workflow
- `plugins/claude-code/skills/status/SKILL.md` — new skill with cycle_status display
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new skill with chunk context loading
- `plugins/claude-code/CLAUDE.md` — replaced with concise full-workflow guide
