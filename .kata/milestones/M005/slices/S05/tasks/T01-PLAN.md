---
estimated_steps: 5
estimated_files: 4
---

# T01: Write plan/status/next-chunk skill files and update CLAUDE.md

**Slice:** S05 — Claude Code Plugin Upgrade
**Milestone:** M005

## Description

Create the three new Claude Code plugin skills (`plan`, `status`, `next-chunk`) and rewrite `CLAUDE.md` to reference the full M005 development cycle workflow. All files are pure markdown. No Rust, no bash. The skills are the user-facing surface that connects the MCP tools built in S01–S04 to Claude Code agent sessions.

`plan/SKILL.md` must follow the interview-first pattern — it instructs the agent to collect goal, chunk names, and criteria from the user conversationally before ever calling `milestone_create` or `spec_create`. Calling tools before gathering input is the primary UX failure mode to avoid.

`status/SKILL.md` is the simplest skill — one `cycle_status` call, two output paths.

`next-chunk/SKILL.md` is the most complex — three sequential tool calls with a null-guard at the `active_chunk_slug` step.

`CLAUDE.md` must stay ≤50 lines (it is injected into every Claude Code conversation) and serve as a reference table, not a tutorial. Detailed instructions live in the skills.

## Steps

1. Create `plugins/claude-code/skills/plan/` directory (the subdirectory is required — existing skills use this pattern) and write `plan/SKILL.md`. The skill must:
   - Have YAML frontmatter with `name: plan` and a description explaining it guides the user through milestone creation
   - Open with a conversational interview step before any MCP tool call: collect feature goal, number of chunks (suggest 2–5), chunk names+slugs, and success criteria per chunk
   - Derive slug as kebab-case from name (describe the pattern: lowercase, spaces→hyphens)
   - After collecting all inputs, call `milestone_create` with `{slug, name, description, chunks: [{slug, name}]}`
   - Call `spec_create` for each chunk with `{slug, name, milestone_slug, criteria}`
   - Confirm results to the user with the created milestone slug and spec paths
   - Follow the exact frontmatter + numbered Steps + Output Format structure of `gate-check/SKILL.md`

2. Create `plugins/claude-code/skills/status/` and write `status/SKILL.md`. The skill must:
   - Have YAML frontmatter with `name: status`
   - Call `cycle_status` (no params required)
   - If response is `{"active": false}`: tell the user no milestone is currently in progress and suggest running `/assay:plan` or `assay milestone list`
   - If response has an active milestone: display milestone name, phase, active chunk slug, and progress (`completed_count / total_count`)
   - Follow the same skill format

3. Create `plugins/claude-code/skills/next-chunk/` and write `next-chunk/SKILL.md`. The skill must:
   - Have YAML frontmatter with `name: next-chunk`
   - Step 1: Call `cycle_status` — if `{"active": false}`, stop and inform the user; if `active_chunk_slug` is null (milestone in Verify phase), tell the user all chunks are complete and suggest `assay pr create <milestone-slug>` or using the `pr_create` tool
   - Step 2: Call `chunk_status` with `{"chunk_slug": "<active_chunk_slug>"}` — display gate pass/fail summary (`passed`, `failed`, `required_failed`); if `has_history: false`, note no gate runs exist yet
   - Step 3: Call `spec_get` with `{"name": "<active_chunk_slug>"}` — display the full spec criteria list (name, description, whether executable, cmd if present)
   - Summarise: active chunk name, gate status, what needs to pass before calling `cycle_advance`
   - Follow the same skill format

4. Rewrite `plugins/claude-code/CLAUDE.md`. Requirements:
   - Total line count ≤50 (including blank lines)
   - A `## Workflow` paragraph describing: plan a milestone with `/assay:plan`, work through chunks using `/assay:next-chunk` to see criteria, run `/assay:gate-check` to verify, call `cycle_advance` when all chunk gates pass, open a PR with `assay pr create <slug>` when all chunks are complete
   - A `## Skills` table with 5 rows: `/assay:spec-show`, `/assay:gate-check`, `/assay:plan`, `/assay:status`, `/assay:next-chunk`
   - A `## MCP Tools` table listing: `spec_list`, `spec_get`, `gate_run`, `milestone_list`, `milestone_get`, `milestone_create`, `spec_create`, `cycle_status`, `cycle_advance`, `chunk_status`, `pr_create`
   - Remove the `## Commands` section that lists only two commands — replace with the 5-skill table above

5. Verify all four files meet their acceptance conditions (see Verification section).

## Must-Haves

- [ ] `plugins/claude-code/skills/plan/SKILL.md` exists with `name: plan` in frontmatter
- [ ] `plugins/claude-code/skills/status/SKILL.md` exists with `name: status` in frontmatter
- [ ] `plugins/claude-code/skills/next-chunk/SKILL.md` exists with `name: next-chunk` in frontmatter
- [ ] All three skills follow YAML frontmatter + `## Steps` + `## Output Format` structure
- [ ] `plan/SKILL.md` interview section appears in Step 1 before any `milestone_create` reference
- [ ] `next-chunk/SKILL.md` contains a guard for `active_chunk_slug: null` telling user to run `assay pr create`
- [ ] `status/SKILL.md` handles `{"active": false}` path explicitly
- [ ] `CLAUDE.md` is ≤50 lines total
- [ ] `CLAUDE.md` has a `## Skills` table with 5 entries (spec-show, gate-check, plan, status, next-chunk)
- [ ] `CLAUDE.md` has a `## MCP Tools` table with all 11 tools

## Verification

```bash
# Files exist
ls plugins/claude-code/skills/plan/SKILL.md
ls plugins/claude-code/skills/status/SKILL.md
ls plugins/claude-code/skills/next-chunk/SKILL.md

# YAML frontmatter — name matches directory
grep '^name: plan$' plugins/claude-code/skills/plan/SKILL.md
grep '^name: status$' plugins/claude-code/skills/status/SKILL.md
grep '^name: next-chunk$' plugins/claude-code/skills/next-chunk/SKILL.md

# Plan skill: interview comes before tool call (line number of interview heading < line number of milestone_create)
INTERVIEW_LINE=$(grep -n 'Interview\|Collect\|Ask\|goal' plugins/claude-code/skills/plan/SKILL.md | head -1 | cut -d: -f1)
TOOL_LINE=$(grep -n 'milestone_create' plugins/claude-code/skills/plan/SKILL.md | head -1 | cut -d: -f1)
[ "$INTERVIEW_LINE" -lt "$TOOL_LINE" ] && echo "OK: interview before tool call" || echo "FAIL: tool called before interview"

# next-chunk: null guard present
grep -c 'null\|pr create\|pr_create' plugins/claude-code/skills/next-chunk/SKILL.md  # ≥1

# CLAUDE.md line count
LINES=$(wc -l < plugins/claude-code/CLAUDE.md)
[ "$LINES" -le 50 ] && echo "OK: $LINES lines" || echo "FAIL: $LINES lines (>50)"

# CLAUDE.md has 5 skills
grep -c '/assay:' plugins/claude-code/CLAUDE.md  # ≥5

# CLAUDE.md has the new MCP tools
grep 'cycle_status' plugins/claude-code/CLAUDE.md
grep 'pr_create' plugins/claude-code/CLAUDE.md
grep 'milestone_create' plugins/claude-code/CLAUDE.md
```

## Observability Impact

- Signals added/changed: None — this task writes documentation/skill files, not runtime code
- How a future agent inspects this: `cat plugins/claude-code/skills/plan/SKILL.md` to see the interview steps; `wc -l plugins/claude-code/CLAUDE.md` to confirm line budget
- Failure state exposed: The `next-chunk` skill's null guard surfaces the "milestone in Verify phase" state to Claude Code users; the `status` skill surfaces `{"active": false}` as a navigable message

## Inputs

- `plugins/claude-code/skills/gate-check/SKILL.md` — template for skill file structure (YAML frontmatter + numbered Steps + Output Format)
- `plugins/claude-code/skills/spec-show/SKILL.md` — second template reference; note `$ARGUMENTS` placeholder usage
- `plugins/claude-code/CLAUDE.md` — current file to be replaced
- S05-RESEARCH.md — MCP tool contracts (params/responses), skill directory structure convention, plan-skill interview-first constraint, next-chunk null guard requirement
- S05-CONTEXT.md — CLAUDE.md ≤50 line constraint; scope decisions; out-of-scope items

## Expected Output

- `plugins/claude-code/skills/plan/SKILL.md` — new skill: interview-first milestone creation workflow (5–8 steps)
- `plugins/claude-code/skills/status/SKILL.md` — new skill: cycle status display (2–3 steps)
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new skill: active chunk context loading (3 steps + null guard)
- `plugins/claude-code/CLAUDE.md` — rewritten: ≤50 lines, 5-skill table, 11-tool table, 1-paragraph workflow
