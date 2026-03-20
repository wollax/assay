---
estimated_steps: 7
estimated_files: 7
---

# T01: Write AGENTS.md and all 5 skill files

**Slice:** S06 — Codex Plugin
**Milestone:** M005

## Description

S06 is a pure-markdown slice. All deliverables are documentation files that connect the Codex agent to Assay's spec-driven development cycle via MCP tools. This single task authors all 6 content files in one pass, anchoring on the confirmed MCP tool signatures from S01–S03. No Rust changes, no compilation, no schema snapshots.

The authoring order matters: AGENTS.md first (sets the vocabulary and tool table that skills reference), then skills in dependency order (simpler ports first, then new tools, interview-first plan last).

Key constraints:
- `AGENTS.md` must stay ≤60 lines — table only the 8 most workflow-relevant tools
- `plan` skill must be interview-first: collect all inputs before calling ANY MCP tool
- `cycle-status` and `next-chunk` are separate skills (overview vs detail) — intentional per user decision in S06-CONTEXT
- `cycle_status` can return `{"active":false}` — all skills that call it must handle this gracefully
- `chunk_status` can return `{"has_history":false}` — skills must not assume `passed`/`failed` fields exist
- Criteria in `spec_create` are text descriptions only; generated specs need manual `cmd` editing before gates are runnable (D076 — mention this in plan skill output)
- Codex has no native hook system — do not reference hooks in AGENTS.md

## Steps

1. **Remove `.gitkeep`** from `plugins/codex/skills/`.

2. **Write `plugins/codex/AGENTS.md`** — ≤60 lines, modeled on `plugins/claude-code/CLAUDE.md` (39-line style reference). Structure:
   - One-paragraph workflow intro
   - Skills table: `/assay:gate-check`, `/assay:spec-show`, `/assay:cycle-status`, `/assay:next-chunk`, `/assay:plan`
   - MCP tools table (8 most relevant): `spec_list`, `spec_get`, `gate_run`, `cycle_status`, `chunk_status`, `milestone_create`, `spec_create`, `milestone_list`
   - Workflow steps (numbered, ≤5 steps): plan → implement → gate-check → advance → PR

3. **Write `plugins/codex/skills/gate-check.md`** — direct port of `plugins/claude-code/skills/gate-check/SKILL.md` with frontmatter added. Preserve the `spec_list` + `gate_run` logic and Output Format (concise pass/fail). Adapt from subdirectory SKILL.md to flat `.md` file.

4. **Write `plugins/codex/skills/spec-show.md`** — direct port of `plugins/claude-code/skills/spec-show/SKILL.md` with frontmatter added. Preserve the `spec_list` + `spec_get` logic and Output Format (group by executable vs descriptive criteria).

5. **Write `plugins/codex/skills/cycle-status.md`** — new skill, overview-first. Steps: (a) call `cycle_status`; if `{"active":false}` print "No active milestone — run `/assay:plan` to create one" and stop; (b) for the active chunk call `chunk_status`; if `has_history:false` show "No gate runs yet — implement the chunk then run `/assay:gate-check <chunk-slug>`"; (c) display: milestone name, phase, X/N chunks complete, active chunk slug, latest pass/fail/required_failed counts. Output Format: concise table.

6. **Write `plugins/codex/skills/next-chunk.md`** — new skill, chunk-detail-first. Steps: (a) call `cycle_status`; if `{"active":false}` stop with guidance; (b) call `chunk_status` for `active_chunk_slug`; (c) call `spec_get` with the active chunk slug to load criteria; (d) display: active chunk slug+name, gate status (pass/fail/pending), then full criteria list so the agent knows exactly what to implement. Output Format: chunk-focused summary + criteria list.

7. **Write `plugins/codex/skills/plan.md`** — new skill, strictly interview-first. Steps must collect ALL inputs before any MCP call:
   - Step 1: Ask for milestone goal, name, and slug (propose a slug from the name)
   - Step 2: Ask for chunk list — names and slugs for each chunk (1–7)
   - Step 3: For each chunk, ask for 1–5 success criteria (text descriptions)
   - Step 4 (only after all inputs): Call `milestone_list` to check for slug collision; warn if slug already exists
   - Step 5: Call `milestone_create` with slug, name, and all chunks
   - Step 6: Call `spec_create` once per chunk with slug, name, criteria list, and `milestone_slug`
   - Output Format: Confirm what was created; list each spec file path; note that `cmd` fields must be manually added to the generated `gates.toml` files before gates are runnable

## Must-Haves

- [ ] `plugins/codex/skills/.gitkeep` deleted
- [ ] `plugins/codex/AGENTS.md` is ≤60 lines
- [ ] `plugins/codex/AGENTS.md` contains a skills table mentioning all 5 skills and an MCP tools table with ≥8 tools
- [ ] `plugins/codex/skills/gate-check.md` has frontmatter with `name: gate-check` and `description:` and calls `spec_list` and `gate_run`
- [ ] `plugins/codex/skills/spec-show.md` has frontmatter with `name: spec-show` and `description:` and calls `spec_list` and `spec_get`
- [ ] `plugins/codex/skills/cycle-status.md` has frontmatter, calls `cycle_status` and `chunk_status`, and has explicit handling for `{"active":false}` and `has_history:false`
- [ ] `plugins/codex/skills/next-chunk.md` has frontmatter, calls `cycle_status`, `chunk_status`, and `spec_get`, handles `{"active":false}` and `has_history:false`
- [ ] `plugins/codex/skills/plan.md` has frontmatter, collects all user inputs (goal, chunks, criteria) in steps 1–3 before calling any MCP tool in steps 4–6, and mentions the `cmd` manual-editing requirement

## Verification

```bash
# Line count check
lines=$(wc -l < plugins/codex/AGENTS.md) && [ "$lines" -le 60 ] && echo "PASS: AGENTS.md $lines lines" || echo "FAIL: AGENTS.md $lines lines (exceeds 60)"

# All skill files exist
for f in gate-check spec-show cycle-status next-chunk plan; do
  [ -f "plugins/codex/skills/$f.md" ] && echo "PASS: $f.md" || echo "FAIL: $f.md missing"
done

# .gitkeep removed
[ ! -f plugins/codex/skills/.gitkeep ] && echo "PASS: .gitkeep removed" || echo "FAIL: .gitkeep still present"

# Tool name correctness
grep -q 'gate_run' plugins/codex/skills/gate-check.md && echo "PASS: gate-check uses gate_run" || echo "FAIL"
grep -q 'spec_get' plugins/codex/skills/spec-show.md && echo "PASS: spec-show uses spec_get" || echo "FAIL"
grep -q 'cycle_status' plugins/codex/skills/cycle-status.md && grep -q 'chunk_status' plugins/codex/skills/cycle-status.md && echo "PASS: cycle-status tools" || echo "FAIL"
grep -q 'cycle_status' plugins/codex/skills/next-chunk.md && grep -q 'spec_get' plugins/codex/skills/next-chunk.md && echo "PASS: next-chunk tools" || echo "FAIL"
grep -q 'milestone_create' plugins/codex/skills/plan.md && grep -q 'spec_create' plugins/codex/skills/plan.md && echo "PASS: plan tools" || echo "FAIL"

# Graceful degradation
grep -qiE 'active.*false|no active|no milestone' plugins/codex/skills/cycle-status.md && echo "PASS: cycle-status handles active:false" || echo "FAIL"
grep -qiE 'has_history|no.*run|no gate run' plugins/codex/skills/next-chunk.md && echo "PASS: next-chunk handles has_history:false" || echo "FAIL"

# Interview-first constraint: milestone_create must not appear before interview steps
# Check that the plan skill has interview language before tool calls
grep -q 'goal\|Goal\|milestone name\|chunk name\|criteria' plugins/codex/skills/plan.md && echo "PASS: plan has interview content" || echo "FAIL"

# cmd editing note in plan skill
grep -qi 'cmd\|command\|manual' plugins/codex/skills/plan.md && echo "PASS: plan mentions cmd editing" || echo "FAIL"
```

## Observability Impact

- Signals added/changed: None — pure documentation
- How a future agent inspects this: `cat plugins/codex/skills/<name>.md`; `wc -l plugins/codex/AGENTS.md`
- Failure state exposed: If a skill fails in Codex, the MCP tool error surfaces directly; skills must handle `{"active":false}` and `{"has_history":false}` to avoid silent failures

## Inputs

- `plugins/claude-code/skills/gate-check/SKILL.md` — port source for gate-check skill
- `plugins/claude-code/skills/spec-show/SKILL.md` — port source for spec-show skill
- `plugins/claude-code/CLAUDE.md` — AGENTS.md style reference (39 lines, three tables)
- S06-RESEARCH.md MCP Tools Reference table — authoritative parameter names and response shapes for all new tools (`cycle_status`, `chunk_status`, `milestone_create`, `spec_create`, `milestone_list`)
- S06-RESEARCH.md `cycle_status` response shapes — exact JSON shapes for `{"active":false}`, `{"has_history":false}`, active milestone response

## Expected Output

- `plugins/codex/AGENTS.md` — new: complete workflow guide, ≤60 lines, three tables + workflow steps
- `plugins/codex/skills/gate-check.md` — new: frontmatter + Steps + Output Format; ports claude-code gate-check
- `plugins/codex/skills/spec-show.md` — new: frontmatter + Steps + Output Format; ports claude-code spec-show
- `plugins/codex/skills/cycle-status.md` — new: frontmatter + Steps + Output Format; cycle overview skill
- `plugins/codex/skills/next-chunk.md` — new: frontmatter + Steps + Output Format; active chunk detail skill
- `plugins/codex/skills/plan.md` — new: frontmatter + Steps + Output Format; interview-first milestone authoring skill
- `plugins/codex/skills/.gitkeep` — deleted
