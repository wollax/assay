# S06: Codex Plugin

**Goal:** Replace the 3-line `plugins/codex/AGENTS.md` stub with a concise workflow reference and add 5 skill files that give Codex users the full Assay development cycle experience inside their agent.
**Demo:** A developer running Codex on this repo can use `/assay:gate-check`, `/assay:spec-show`, `/assay:cycle-status`, `/assay:next-chunk`, and `/assay:plan` to drive the spec-driven workflow; `plugins/codex/AGENTS.md` gives a complete workflow overview in ≤60 lines; all skill files are self-contained and include correct MCP tool names and parameters.

## Must-Haves

- `plugins/codex/AGENTS.md` exists, is ≤60 lines, and contains skills table, MCP tools table, and workflow steps
- `plugins/codex/skills/gate-check.md` calls `spec_list` + `gate_run` with correct parameter names
- `plugins/codex/skills/spec-show.md` calls `spec_list` + `spec_get` with correct parameter names
- `plugins/codex/skills/cycle-status.md` calls `cycle_status` + `chunk_status` and handles `{"active":false}` gracefully
- `plugins/codex/skills/next-chunk.md` calls `cycle_status` + `chunk_status` + `spec_get` and handles missing chunk history
- `plugins/codex/skills/plan.md` collects all user inputs (goal, chunks, criteria) before calling any MCP tool; calls `milestone_create` + `spec_create` only after the interview is complete
- `plugins/codex/skills/.gitkeep` is removed
- All skill files use the standard frontmatter + `## Steps` + `## Output Format` structure

## Proof Level

- This slice proves: contract (content correctness verified by grep assertions + manual structural inspection)
- Real runtime required: no (no Rust compilation; no agent execution needed)
- Human/UAT required: yes (actually using the plugin in Codex is UAT-only)

## Verification

```bash
# AGENTS.md line count ≤60
lines=$(wc -l < plugins/codex/AGENTS.md) && [ "$lines" -le 60 ] && echo "PASS: $lines lines" || echo "FAIL: $lines lines"

# All 5 skill files exist
for f in gate-check spec-show cycle-status next-chunk plan; do
  [ -f "plugins/codex/skills/$f.md" ] && echo "PASS: $f.md" || echo "FAIL: $f.md missing"
done

# .gitkeep removed
[ ! -f plugins/codex/skills/.gitkeep ] && echo "PASS: .gitkeep removed" || echo "FAIL: .gitkeep still present"

# gate-check: uses spec_list and gate_run
grep -q 'spec_list' plugins/codex/skills/gate-check.md && grep -q 'gate_run' plugins/codex/skills/gate-check.md && echo "PASS: gate-check tools" || echo "FAIL"

# spec-show: uses spec_list and spec_get
grep -q 'spec_list' plugins/codex/skills/spec-show.md && grep -q 'spec_get' plugins/codex/skills/spec-show.md && echo "PASS: spec-show tools" || echo "FAIL"

# cycle-status: uses cycle_status and chunk_status; handles active:false
grep -q 'cycle_status' plugins/codex/skills/cycle-status.md && grep -q 'chunk_status' plugins/codex/skills/cycle-status.md && grep -q 'active.*false\|no active\|no milestone' plugins/codex/skills/cycle-status.md && echo "PASS: cycle-status" || echo "FAIL"

# next-chunk: uses cycle_status, chunk_status, spec_get
grep -q 'cycle_status' plugins/codex/skills/next-chunk.md && grep -q 'chunk_status' plugins/codex/skills/next-chunk.md && grep -q 'spec_get' plugins/codex/skills/next-chunk.md && echo "PASS: next-chunk tools" || echo "FAIL"

# plan: interview-first — milestone_create must not appear before all inputs are gathered
# Verify milestone_create only appears in later steps (after user input collection steps)
grep -q 'milestone_create' plugins/codex/skills/plan.md && grep -q 'spec_create' plugins/codex/skills/plan.md && echo "PASS: plan tools present" || echo "FAIL"
grep -q 'goal\|Goal\|milestone name\|chunk' plugins/codex/skills/plan.md && echo "PASS: plan has interview" || echo "FAIL"

# AGENTS.md references all 5 skills
for skill in gate-check spec-show cycle-status next-chunk plan; do
  grep -q "$skill" plugins/codex/AGENTS.md && echo "PASS: AGENTS.md mentions $skill" || echo "FAIL: $skill missing from AGENTS.md"
done
```

## Observability / Diagnostics

- Runtime signals: none (pure markdown; no runtime state)
- Inspection surfaces: `wc -l plugins/codex/AGENTS.md`; `cat plugins/codex/skills/<name>.md`
- Failure visibility: if a skill fails in Codex, the agent will surface the MCP tool error; skill files must handle `{"active":false}` and `{"has_history":false}` cases to avoid silent failure
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_list`, `milestone_create`, `spec_create`, `spec_list`, `spec_get`, `gate_run` MCP tools (all delivered in S01–S03)
- New wiring introduced in this slice: skill files installed via symlink per README (`ln -s .../plugins/codex/skills .agents/skills/assay`); AGENTS.md auto-loaded by Codex as agent instructions
- What remains before the milestone is truly usable end-to-end: human installs the plugin and exercises it in Codex (UAT)

## Tasks

- [x] **T01: Write AGENTS.md and all 5 skill files** `est:45m`
  - Why: S06 is a pure content slice — all 6 files are independent markdown artifacts designed in a single authoring pass using the confirmed MCP tool signatures from S01–S03
  - Files: `plugins/codex/AGENTS.md`, `plugins/codex/skills/gate-check.md`, `plugins/codex/skills/spec-show.md`, `plugins/codex/skills/cycle-status.md`, `plugins/codex/skills/next-chunk.md`, `plugins/codex/skills/plan.md`
  - Do: (1) Remove `.gitkeep`; (2) Write `AGENTS.md` first — ≤60 lines, three tables (skills, MCP tools, workflow steps), modeled on `plugins/claude-code/CLAUDE.md`; (3) Write `gate-check.md` — port from `plugins/claude-code/skills/gate-check/SKILL.md`, add frontmatter, preserve `spec_list`+`gate_run` logic; (4) Write `spec-show.md` — port from `plugins/claude-code/skills/spec-show/SKILL.md`, add frontmatter, preserve `spec_list`+`spec_get` logic; (5) Write `cycle-status.md` — calls `cycle_status` (overview), then `chunk_status` for active chunk, handles `{"active":false}` by suggesting `/assay:plan`; (6) Write `next-chunk.md` — calls `cycle_status` first, then `chunk_status` for gate detail, then `spec_get` for criteria; handles `has_history:false` gracefully; (7) Write `plan.md` — strict interview-first: collect (a) milestone goal/name/slug, (b) chunk list with slugs+names, (c) criteria per chunk — only THEN call `milestone_create` followed by `spec_create` per chunk; mention that generated specs need manual `cmd` editing before gates run
  - Verify: run the shell assertions in the Verification section above; all must print PASS
  - Done when: all 5 skill files exist, `.gitkeep` removed, `AGENTS.md` is ≤60 lines, all grep checks pass

## Files Likely Touched

- `plugins/codex/AGENTS.md`
- `plugins/codex/skills/gate-check.md`
- `plugins/codex/skills/spec-show.md`
- `plugins/codex/skills/cycle-status.md`
- `plugins/codex/skills/next-chunk.md`
- `plugins/codex/skills/plan.md`
- `plugins/codex/skills/.gitkeep` (deleted)
