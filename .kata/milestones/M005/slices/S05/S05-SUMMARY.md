---
id: S05
parent: M005
milestone: M005
provides:
  - plugins/claude-code/skills/plan/SKILL.md — interview-first milestone creation skill (/assay:plan)
  - plugins/claude-code/skills/status/SKILL.md — cycle status display skill (/assay:status)
  - plugins/claude-code/skills/next-chunk/SKILL.md — active chunk context loading skill (/assay:next-chunk)
  - plugins/claude-code/CLAUDE.md — rewritten with 5-skill table, 11-tool table, 1-paragraph workflow guide
  - plugins/claude-code/scripts/cycle-stop-check.sh — cycle-aware Stop hook with 7 guards and per-chunk gate evaluation
  - plugins/claude-code/scripts/post-tool-use.sh — updated to surface active chunk name in reminder text
  - plugins/claude-code/hooks/hooks.json — Stop hook wired to cycle-stop-check.sh (stop-gate-check.sh removed)
  - plugins/claude-code/.claude-plugin/plugin.json — version bumped to 0.5.0
requires:
  - slice: S01
    provides: milestone_list, milestone_get MCP tools; Milestone type; milestone TOML format
  - slice: S02
    provides: cycle_status, cycle_advance, chunk_status MCP tools; assay milestone status CLI
  - slice: S03
    provides: milestone_create, spec_create MCP tools
  - slice: S04
    provides: pr_create MCP tool; assay pr create CLI
affects: []
key_files:
  - plugins/claude-code/skills/plan/SKILL.md
  - plugins/claude-code/skills/status/SKILL.md
  - plugins/claude-code/skills/next-chunk/SKILL.md
  - plugins/claude-code/CLAUDE.md
  - plugins/claude-code/scripts/cycle-stop-check.sh
  - plugins/claude-code/scripts/post-tool-use.sh
  - plugins/claude-code/hooks/hooks.json
  - plugins/claude-code/.claude-plugin/plugin.json
key_decisions:
  - D080: skill interview-first pattern — all user inputs collected conversationally before any MCP tool call
  - D081: next-chunk handles active_chunk_slug=null (Verify phase) with explicit "run assay pr create" hint
  - D082: guard ordering in cycle-stop-check.sh: jq → stop_hook_active → MODE → .assay/ dir → binary → cd → work
  - D083: BLOCKING_CHUNKS named verbatim in Stop hook block reason for immediate actionability
patterns_established:
  - skill interview-first pattern: conversational input collection → confirm summary → MCP tool calls
  - skill null-guard pattern: check cycle_status.active before proceeding; check active_chunk_slug for Verify phase
  - cycle-aware stop hook: discover incomplete chunks → per-chunk gate checks → accumulate BLOCKING_CHUNKS → name them in block reason
observability_surfaces:
  - Stop hook block output: `{ decision: "block", reason: "... in chunks: <slug> ..." }` — agent reads BLOCKING_CHUNKS to target /assay:gate-check
  - Warn mode: `{ systemMessage: "Warning: ... in chunks: ..." }` via ASSAY_STOP_HOOK_MODE=warn
  - PostToolUse: additionalContext names active chunk slug when present
  - ASSAY_STOP_HOOK_MODE env var: enforce (default) | warn | off
drill_down_paths:
  - .kata/milestones/M005/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M005/slices/S05/tasks/T02-SUMMARY.md
duration: 30min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
---

# S05: Claude Code Plugin Upgrade

**Three new skills (`/assay:plan`, `/assay:status`, `/assay:next-chunk`), a rewritten CLAUDE.md, and a cycle-aware Stop hook complete the Claude Code integration surface for the M005 guided development cycle.**

## What Happened

**T01** created three new skill files under `plugins/claude-code/skills/` and rewrote `CLAUDE.md`.

`plan/SKILL.md` uses an interview-first pattern: Step 1 collects goal, chunk count, chunk names/slugs, and criteria per chunk conversationally; Step 2 calls `milestone_create` + `spec_create` per chunk only after confirming a summary with the user. This ordering prevents orphan milestone files from abandoned interviews.

`status/SKILL.md` is minimal: call `cycle_status`, branch on `active: false` (no milestone) vs. active milestone to display name/phase/chunk/progress.

`next-chunk/SKILL.md` chains three tool calls (`cycle_status` → `chunk_status` → `spec_get`) with two null guards: one for `active: false` (no active milestone) and one for `active_chunk_slug: null` (Verify phase — all chunks done, user should run `assay pr create`). The Verify-phase guard closes the workflow loop explicitly.

`CLAUDE.md` was rewritten to 33 lines: one workflow paragraph, a 5-row Skills table (`/assay:gate-check`, `/assay:spec-show`, `/assay:plan`, `/assay:status`, `/assay:next-chunk`), and an 11-row MCP Tools table covering all 8 new M005 tools plus the 3 existing ones.

**T02** wrote `cycle-stop-check.sh` by extending the existing 7-guard pattern from `stop-gate-check.sh` with cycle-aware logic. The additions: detect incomplete chunk slugs via `assay milestone status 2>/dev/null | grep '\[ \]' | awk '{print $2}'`; run `gate run "$chunk" --json` per incomplete chunk and accumulate `FAILED_COUNT` and `BLOCKING_CHUNKS`; if no active milestone, fall back to `gate run --all --json` (original behavior). Both block and warn paths include `$BLOCKING_CHUNKS` in the output so the agent can immediately target the specific failing chunk.

`post-tool-use.sh` was updated to detect the first incomplete chunk and embed it in `additionalContext`. Graceful degradation is implicit — empty `ACTIVE_CHUNK` falls back to the original generic message.

`hooks.json` Stop hook was updated from `stop-gate-check.sh` to `cycle-stop-check.sh` (one line). `plugin.json` version bumped to `0.5.0`.

## Verification

```
# T01
ls plugins/claude-code/skills/{plan,status,next-chunk}/SKILL.md  → all exist
YAML frontmatter with name: field in each skill                   → ✓
Skill names match directory names (plan/status/next-chunk)        → ✓
Interview-first: interview section at line 6, milestone_create at line 24 → ✓
CLAUDE.md line count: 33 (≤50 budget)                            → ✓
/assay: occurrences in CLAUDE.md: 6 (≥5 required)               → ✓
cycle_status and pr_create in CLAUDE.md                          → ✓

# T02
bash -n cycle-stop-check.sh   → OK (no syntax errors)
bash -n post-tool-use.sh      → OK (no syntax errors)
jq . hooks.json               → valid JSON
grep 'cycle-stop-check' hooks.json → present
grep -c 'stop-gate-check' hooks.json → 0
plugin.json version "0.5.0"   → ✓
grep -c 'exit 0' cycle-stop-check.sh → 11 (≥7 required)
```

## Requirements Advanced

- R047 (Claude Code plugin upgrade) — all deliverables complete: 3 new skills, updated CLAUDE.md, cycle-aware Stop hook, PostToolUse update, hooks.json wired, version 0.5.0

## Requirements Validated

- R047 — validated: all skill files exist with correct frontmatter and content; CLAUDE.md ≤50 lines with all required references; cycle-stop-check.sh passes bash -n with ≥7 exit-0 guards; hooks.json references cycle-stop-check.sh only; plugin.json at 0.5.0

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

None. Both tasks implemented exactly per the slice plan.

## Known Limitations

- Live Claude Code session verification is UAT-only — skill rendering, hook blocking behavior, and MCP round-trips require a real Claude Code session (see S05-UAT.md)
- `stop-gate-check.sh` remains on disk (not deleted) — it is no longer referenced by hooks.json but preserved for reference

## Follow-ups

- S06 (Codex plugin) ports `gate-check` and `spec-show` skills and adds `cycle-status` and `plan` skills using the same patterns established here

## Files Created/Modified

- `plugins/claude-code/skills/plan/SKILL.md` — new: interview-first milestone creation skill (5 steps)
- `plugins/claude-code/skills/status/SKILL.md` — new: cycle status display skill (2 steps)
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new: active chunk context loading skill (3 steps + 2 null guards)
- `plugins/claude-code/CLAUDE.md` — rewritten: 33 lines, 5-skill table, 11-tool table, workflow paragraph
- `plugins/claude-code/scripts/cycle-stop-check.sh` — new: 7-guard cycle-aware Stop hook with per-chunk gate evaluation
- `plugins/claude-code/scripts/post-tool-use.sh` — updated: additionalContext names active chunk slug when present
- `plugins/claude-code/hooks/hooks.json` — updated: Stop hook points to cycle-stop-check.sh
- `plugins/claude-code/.claude-plugin/plugin.json` — updated: version 0.4.0 → 0.5.0

## Forward Intelligence

### What the next slice should know
- S06 (Codex plugin) can directly port the `plan/SKILL.md` interview-first pattern; the only change needed is removing the Claude Code skill frontmatter format requirements and using Codex-compatible SKILL.md format
- The `cycle_status` → `chunk_status` → `spec_get` chain in `next-chunk` is the canonical "show me what to do next" pattern — S06 should replicate it verbatim
- `CLAUDE.md` is now at 33/50 lines — there is headroom for one more table if S06 adds cross-plugin instructions

### What's fragile
- `assay milestone status` parsing in cycle-stop-check.sh uses `grep '\[ \]'` + `awk '{print $2}'` — this depends on the exact output format of `assay milestone status`; if that format changes, the stop hook silently falls back to `--all` mode without error

### Authoritative diagnostics
- `ASSAY_STOP_HOOK_MODE=warn bash plugins/claude-code/scripts/cycle-stop-check.sh <<< '{}'` — tests warn-mode output outside Claude Code
- `grep -c 'exit 0' plugins/claude-code/scripts/cycle-stop-check.sh` — confirms guard count (should be ≥11)

### What assumptions changed
- No assumptions changed — this slice was pure plugin content with no core library changes
