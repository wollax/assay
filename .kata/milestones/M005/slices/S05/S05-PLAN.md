# S05: Claude Code Plugin Upgrade

**Goal:** Upgrade the Claude Code plugin to expose the full M005 development cycle — `/assay:plan`, `/assay:status`, and `/assay:next-chunk` skills; a cycle-aware Stop hook; a cycle-aware PostToolUse reminder; and an updated CLAUDE.md that describes the complete plan→execute→verify→PR loop.
**Demo:** A Claude Code user invokes `/assay:plan`, answers conversational questions about goal and chunks, and the plugin calls `milestone_create` + `spec_create` to create the milestone and specs. `/assay:status` shows active milestone/chunk/phase. `/assay:next-chunk` shows the active chunk's criteria and gate status. The Stop hook blocks completion when an active chunk has failing gates, naming the blocking chunk.

## Must-Haves

- `plugins/claude-code/skills/plan/SKILL.md` exists with YAML frontmatter, interview-first steps (goal → chunks → criteria before calling MCP tools), and instructions for calling `milestone_create` + `spec_create`
- `plugins/claude-code/skills/status/SKILL.md` exists with YAML frontmatter and calls `cycle_status`; handles both `{"active": false}` and active-milestone paths
- `plugins/claude-code/skills/next-chunk/SKILL.md` exists with YAML frontmatter and executes the three-tool chain (`cycle_status` → `chunk_status` → `spec_get`); handles `active_chunk_slug: null` gracefully (tells user to run `assay pr create`)
- `plugins/claude-code/CLAUDE.md` is ≤50 lines, lists all 5 skills in a command table, lists all 8 new MCP tools plus the 3 existing tools, and includes a 1-paragraph guided workflow summary
- `plugins/claude-code/scripts/cycle-stop-check.sh` implements all 7 safety guards from `stop-gate-check.sh` plus cycle-aware gate checking: detects active incomplete chunks via `assay milestone status`, runs per-chunk gate checks, falls back to `--all` when no active milestone
- `plugins/claude-code/scripts/post-tool-use.sh` detects active chunk via `assay milestone status` and names it in the reminder message; falls back to generic gate-check reminder when no milestone is active
- `plugins/claude-code/hooks/hooks.json` references `cycle-stop-check.sh` (not `stop-gate-check.sh`) in the Stop hook
- `plugins/claude-code/.claude-plugin/plugin.json` version bumped to `0.5.0`
- `bash -n` passes on both `cycle-stop-check.sh` and `post-tool-use.sh`

## Proof Level

- This slice proves: integration — all 8 S01–S04 MCP tools are wired into the Claude Code plugin surface via skills and a cycle-aware hook
- Real runtime required: no (bash syntax check + file existence is sufficient for automated verification; actual Claude Code session is UAT-only)
- Human/UAT required: yes — live Claude Code session needed to verify skill rendering, hook blocking behavior, and MCP round-trips

## Verification

```bash
# T01 verification — skills and CLAUDE.md
ls plugins/claude-code/skills/plan/SKILL.md
ls plugins/claude-code/skills/status/SKILL.md
ls plugins/claude-code/skills/next-chunk/SKILL.md

# YAML frontmatter present in each skill
head -5 plugins/claude-code/skills/plan/SKILL.md | grep -c 'name:'
head -5 plugins/claude-code/skills/status/SKILL.md | grep -c 'name:'
head -5 plugins/claude-code/skills/next-chunk/SKILL.md | grep -c 'name:'

# Skill names match directory names
grep -r '^name: plan$' plugins/claude-code/skills/plan/SKILL.md
grep -r '^name: status$' plugins/claude-code/skills/status/SKILL.md
grep -r '^name: next-chunk$' plugins/claude-code/skills/next-chunk/SKILL.md

# Plan skill is interview-first (milestone_create must not appear before the interview heading)
awk '/^#/{section=$0} /milestone_create/{print section}' plugins/claude-code/skills/plan/SKILL.md | head -1
# Expected: the section heading that contains milestone_create should be a "Call MCP" step, not the opening step

# CLAUDE.md line count ≤50
wc -l plugins/claude-code/CLAUDE.md

# CLAUDE.md references all 5 skills
grep -c '/assay:' plugins/claude-code/CLAUDE.md  # ≥5

# CLAUDE.md references cycle_status and pr_create MCP tools
grep 'cycle_status' plugins/claude-code/CLAUDE.md
grep 'pr_create' plugins/claude-code/CLAUDE.md

# T02 verification — scripts, hooks, version
bash -n plugins/claude-code/scripts/cycle-stop-check.sh
bash -n plugins/claude-code/scripts/post-tool-use.sh

# Valid JSON
jq . plugins/claude-code/hooks/hooks.json >/dev/null

# hooks.json uses new hook script
grep 'cycle-stop-check' plugins/claude-code/hooks/hooks.json
# stop-gate-check.sh must NOT be referenced
grep -c 'stop-gate-check' plugins/claude-code/hooks/hooks.json  # must be 0

# Version bumped
grep '"0.5.0"' plugins/claude-code/.claude-plugin/plugin.json

# 7 safety guards present in cycle-stop-check.sh (count guard-exit paths)
grep -c 'exit 0' plugins/claude-code/scripts/cycle-stop-check.sh  # ≥7
```

## Observability / Diagnostics

This slice is pure plugin content — it surfaces the observability already built in S01–S04.

- Runtime signals: Stop hook outputs `decision: "block"` + `reason` JSON naming the failing chunk slug and failing criteria count; warn mode outputs `systemMessage` with same info; PostToolUse outputs `additionalContext` naming the active chunk
- Inspection surfaces: `ASSAY_STOP_HOOK_MODE=warn|off|enforce` env var controls Stop hook behavior; `assay milestone status` CLI surfaces active milestone/chunk state outside of Claude Code
- Failure visibility: `cycle-stop-check.sh` names the specific blocking chunk slug in its reason message, allowing the agent to immediately invoke `/assay:gate-check <chunk-slug>` to diagnose
- Redaction constraints: none — milestone slugs and chunk slugs are non-sensitive identifiers

## Integration Closure

- Upstream surfaces consumed: `milestone_create`, `spec_create`, `cycle_status`, `chunk_status`, `cycle_advance`, `pr_create`, `spec_get`, `gate_run` MCP tools (all registered in S01–S04); `assay milestone status` CLI (S02); `stop-gate-check.sh` guard pattern (existing)
- New wiring introduced in this slice: three skill files hook up to the new MCP tools; `cycle-stop-check.sh` replaces `stop-gate-check.sh` in `hooks.json`; updated `post-tool-use.sh` adds cycle awareness
- What remains before the milestone is truly usable end-to-end: nothing — S05 is the final Claude Code surface layer; S06 (Codex) is a parallel slice using the same MCP tools

## Tasks

- [x] **T01: Write plan/status/next-chunk skill files and update CLAUDE.md** `est:45m`
  - Why: Creates the three new skills and updates the workflow guide — the visible user-facing surface of the entire M005 upgrade for Claude Code users
  - Files: `plugins/claude-code/skills/plan/SKILL.md`, `plugins/claude-code/skills/status/SKILL.md`, `plugins/claude-code/skills/next-chunk/SKILL.md`, `plugins/claude-code/CLAUDE.md`
  - Do: Create `plan/` subdirectory under skills; write `plan/SKILL.md` with interview-first steps (Step 1: collect goal, Step 2: collect chunks+criteria, Step 3: call `milestone_create` + `spec_create` per chunk, Step 4: confirm and advance); create `status/SKILL.md` calling `cycle_status` with both active and `{"active":false}` handling; create `next-chunk/SKILL.md` calling `cycle_status` → `chunk_status` → `spec_get` chain with null `active_chunk_slug` guard (tell user to run `assay pr create`); rewrite `CLAUDE.md` to include 5-skill command table + all 11 MCP tools table + 1-paragraph workflow summary; keep ≤50 lines
  - Verify: `ls plugins/claude-code/skills/plan/SKILL.md` && `ls plugins/claude-code/skills/status/SKILL.md` && `ls plugins/claude-code/skills/next-chunk/SKILL.md`; `wc -l plugins/claude-code/CLAUDE.md` ≤50; `grep -c '/assay:' plugins/claude-code/CLAUDE.md` ≥5; `grep 'milestone_create' plugins/claude-code/skills/plan/SKILL.md` appears after the interview section heading
  - Done when: All 3 skill files exist with valid YAML frontmatter and name matching directory; CLAUDE.md is ≤50 lines and references all 5 skills and key MCP tools

- [ ] **T02: Write cycle-stop-check.sh, update post-tool-use.sh, update hooks.json and plugin.json** `est:45m`
  - Why: Replaces the existing gate-only Stop hook with a cycle-aware version that names blocking chunks; adds cycle context to the PostToolUse reminder; bumps plugin version to reflect M005 capabilities
  - Files: `plugins/claude-code/scripts/cycle-stop-check.sh`, `plugins/claude-code/scripts/post-tool-use.sh`, `plugins/claude-code/hooks/hooks.json`, `plugins/claude-code/.claude-plugin/plugin.json`
  - Do: Write `cycle-stop-check.sh` with all 7 guards from `stop-gate-check.sh` (jq, stop_hook_active, MODE, .assay/ dir, binary) plus 2 cycle-specific steps: detect active incomplete chunks via `assay milestone status 2>/dev/null | grep '\[ \]' | awk '{print $2}'`; if active chunks found, run `assay gate run "$chunk" --json` for each and aggregate failures; if no active milestone, fall back to `assay gate run --all --json`; block/warn/allow based on `ASSAY_STOP_HOOK_MODE`; update `post-tool-use.sh` to detect active chunk via `assay milestone status 2>/dev/null | grep '\[ \]' | awk 'NR==1{print $2}'` and include it in the reminder message with reference to `/assay:next-chunk`; edit `hooks.json` Stop array to reference `cycle-stop-check.sh` instead of `stop-gate-check.sh`; bump `plugin.json` version to `0.5.0`
  - Verify: `bash -n plugins/claude-code/scripts/cycle-stop-check.sh`; `bash -n plugins/claude-code/scripts/post-tool-use.sh`; `jq . plugins/claude-code/hooks/hooks.json >/dev/null`; `grep cycle-stop-check plugins/claude-code/hooks/hooks.json`; `grep -c stop-gate-check plugins/claude-code/hooks/hooks.json` = 0; `grep '"0.5.0"' plugins/claude-code/.claude-plugin/plugin.json`
  - Done when: Both scripts pass `bash -n`; hooks.json is valid JSON referencing `cycle-stop-check.sh`; plugin.json shows version `0.5.0`; `stop-gate-check.sh` is no longer referenced in hooks.json

## Files Likely Touched

- `plugins/claude-code/skills/plan/SKILL.md` — new
- `plugins/claude-code/skills/status/SKILL.md` — new
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new
- `plugins/claude-code/CLAUDE.md` — rewritten
- `plugins/claude-code/scripts/cycle-stop-check.sh` — new
- `plugins/claude-code/scripts/post-tool-use.sh` — updated
- `plugins/claude-code/hooks/hooks.json` — updated
- `plugins/claude-code/.claude-plugin/plugin.json` — version bump
