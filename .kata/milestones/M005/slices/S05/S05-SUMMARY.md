---
id: S05
parent: M005
milestone: M005
provides:
  - --json flag on `assay milestone status` outputting CycleStatus JSON or {"active":false}
  - plugins/claude-code/skills/plan/SKILL.md — interview-first milestone+spec creation skill
  - plugins/claude-code/skills/status/SKILL.md — cycle status display skill
  - plugins/claude-code/skills/next-chunk/SKILL.md — active chunk context loader skill
  - plugins/claude-code/CLAUDE.md — updated with full workflow reference (5 skills + 5 CLI + 11 MCP tools)
  - plugins/claude-code/scripts/cycle-stop-check.sh — cycle-aware Stop hook scoped to active chunk
  - plugins/claude-code/scripts/post-tool-use.sh — updated with active chunk name in reminder message
  - plugins/claude-code/hooks/hooks.json — Stop[0] wired to cycle-stop-check.sh
  - plugins/claude-code/.claude-plugin/plugin.json — version bumped to 0.5.0
  - Cargo.toml workspace version bumped to 0.5.0 (required by check-plugin-version guard)
requires:
  - slice: S01
    provides: Milestone, ChunkRef, MilestoneStatus types; milestone_load/save/scan; milestone_list/milestone_get MCP tools
  - slice: S02
    provides: cycle_status, cycle_advance, chunk_status MCP tools; CycleStatus type; assay milestone status/advance CLI
  - slice: S03
    provides: milestone_create, spec_create MCP tools; assay plan wizard
  - slice: S04
    provides: pr_create MCP tool; assay pr create CLI; pr_check_milestone_gates logic
affects:
  - S06 (Codex plugin — consumes same MCP tool surface; not blocked by S05)
key_files:
  - crates/assay-cli/src/commands/milestone.rs
  - plugins/claude-code/skills/plan/SKILL.md
  - plugins/claude-code/skills/status/SKILL.md
  - plugins/claude-code/skills/next-chunk/SKILL.md
  - plugins/claude-code/CLAUDE.md
  - plugins/claude-code/scripts/cycle-stop-check.sh
  - plugins/claude-code/scripts/post-tool-use.sh
  - plugins/claude-code/hooks/hooks.json
  - plugins/claude-code/.claude-plugin/plugin.json
  - Cargo.toml
key_decisions:
  - D080 — `{"active":false}` detection in bash via `jq 'has("milestone_slug")'` (not `.active` key)
  - D081 — `assay milestone status --json` exits 0 always; only I/O errors produce exit 1
patterns_established:
  - Cycle-aware hook pattern: call `assay milestone status --json`, detect active milestone via `jq 'has("milestone_slug")'`, extract `active_chunk_slug`, scope gate run to chunk with fallback to `--all`
  - Skill interview-first pattern: /assay:plan always collects all inputs conversationally before any MCP calls (never calls milestone_create on invocation)
  - Shell message interpolation in Claude hook JSON output: use `jq -n --arg msg "$VAR"` rather than heredoc when message contains a dynamic shell variable
  - JSON output branch in CLI commands: check `--json` flag first, call domain fn, serialize, return early (established in T01)
observability_surfaces:
  - "`assay milestone status --json | jq .` — shows active CycleStatus or {\"active\":false}; exits 0 always; non-zero only on I/O error to stderr"
  - "Stop hook outputs `{ decision: \"block\", reason: \"Quality gates failing for chunk '<slug>' (N criteria). Run /assay:gate-check <slug> for details.\" }` in enforce mode"
  - "Stop hook outputs `{ systemMessage: \"Warning: quality gates are failing for chunk '<slug>'...\" }` in warn mode"
  - "PostToolUse reminder appends \" Active chunk: <slug>.\" when a milestone is in_progress — visible in Claude's context after each file write/edit"
drill_down_paths:
  - .kata/milestones/M005/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M005/slices/S05/tasks/T02-SUMMARY.md
  - .kata/milestones/M005/slices/S05/tasks/T03-SUMMARY.md
duration: ~55 minutes total (T01: ~10m, T02: ~15m, T03: ~30m)
verification_result: passed
completed_at: 2026-03-20
---

# S05: Claude Code Plugin Upgrade

**Upgraded the Claude Code plugin to a full cycle-aware development surface: three new skills (`/assay:plan`, `/assay:status`, `/assay:next-chunk`), updated CLAUDE.md, a cycle-scoped Stop hook, an active-chunk-aware PostToolUse reminder, and version 0.5.0.**

## What Happened

Three tasks executed sequentially, each building on the preceding:

**T01 — `--json` flag on `assay milestone status`:** Added `#[arg(long)] json: bool` to `MilestoneCommand::Status` and branched `milestone_status_cmd` to call `assay_core::milestone::cycle_status(&dir)` and serialize: `Ok(None)` → `{"active":false}`, `Ok(Some(s))` → full CycleStatus JSON, `Err(e)` → eprintln + `Ok(1)` (D072). The key structural decision (D080) was that the sentinel `{"active":false}` lacks `milestone_slug` entirely — bash hooks detect active milestones via `jq 'has("milestone_slug")'`, not by checking a `.active` boolean key. Added `milestone_status_json_no_active` test; updated existing `milestone_status_no_milestones` test for new variant shape.

**T02 — Skills and CLAUDE.md:** Created three skill files following the `gate-check/SKILL.md` format (frontmatter + Steps + Output Format). `plan/SKILL.md` is interview-first: Step 1 collects milestone goal, chunk count, and per-chunk slug/name/criteria conversationally before any MCP calls; Steps 2-4 call `milestone_create` then `spec_create` per chunk and warn about missing `cmd` fields. `status/SKILL.md` calls `cycle_status`, handles the `{"active":false}` sentinel, and renders a `[x][ ][ ]` progress display. `next-chunk/SKILL.md` chains `cycle_status` → `chunk_status` → `spec_get` and presents criteria with ✓/✗ pass/fail indicators. CLAUDE.md replaced with a concise 39-line three-table reference (Skills, CLI Commands, MCP Tools).

**T03 — Hook infrastructure, plugin version:** Created `cycle-stop-check.sh` by transplanting guards 1-5 verbatim from `stop-gate-check.sh` and inserting cycle-aware logic between guards 4 and 5. The new block calls `assay milestone status --json`, uses `jq 'has("milestone_slug")'` to detect an active milestone, extracts `active_chunk_slug`, and scopes `assay gate run` to that chunk (fallback `--all`). Updated `post-tool-use.sh` to append the active chunk name to the additionalContext reminder, switching from heredoc to `jq -n --arg` for dynamic variable interpolation. Updated `hooks.json` Stop[0] command; bumped `plugin.json` to `0.5.0`. Discovered the `check-plugin-version` just recipe enforces exact match between workspace Cargo.toml version and plugin.json — bumped workspace version to `0.5.0` as well.

## Verification

- `cargo test -p assay-cli -- milestone_status_json` — 1 test passed (`milestone_status_json_no_active`)
- `cargo test --workspace` — 1331+ tests, 0 failures
- `just ready` — fmt + clippy + test + deny all green, "All checks passed." (0.5.0 plugin version match confirmed)
- `bash -n plugins/claude-code/scripts/cycle-stop-check.sh` — no syntax errors
- `bash -n plugins/claude-code/scripts/post-tool-use.sh` — no syntax errors
- Content checks: `grep -l "milestone_create|spec_create" plan/SKILL.md` ✓, `grep "cycle_status" status/SKILL.md` ✓, `grep "chunk_status" next-chunk/SKILL.md` ✓, `grep "assay:plan" CLAUDE.md` ✓
- `grep '"version": "0.5.0"' plugin.json` ✓
- `grep "cycle-stop-check.sh" hooks.json` ✓

## Requirements Advanced

- R047 (Claude Code plugin upgrade) — was `active`; this slice delivers all deliverables: 3 new skills, updated CLAUDE.md, cycle-aware Stop hook, updated PostToolUse, hooks.json wiring, version bump → moved to `validated`

## Requirements Validated

- R047 — All specified deliverables confirmed present and verified: skills invoke correct MCP tools, bash hooks have no syntax errors, `just ready` green with 1331+ tests

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- **Workspace Cargo.toml version bumped to 0.5.0** (not in S05-PLAN.md tasks): the `check-plugin-version` just recipe enforces exact match between workspace version and plugin.json. Bumping only plugin.json would fail `just ready`. Correct fix per project versioning convention.
- **`jq 'has("milestone_slug")'` instead of `.active` field check**: The sentinel `{"active":false}` returned by the `--json` flag does not include an `.active` key in the `CycleStatus` struct — it is a completely separate JSON object. Detection must check for the presence of `milestone_slug` (a field that only exists in real CycleStatus responses). This structural detail was not documented in the original task plan but was addressed in S05-RESEARCH.md guidance.
- **`jq -n --arg` instead of heredoc in post-tool-use.sh**: heredoc cannot interpolate `$ACTIVE_CHUNK_MSG` shell variable — the variable would be emitted literally. `jq --arg` is the correct pattern for dynamic JSON construction in bash.

## Known Limitations

- **UAT is human-required**: Interactive skill invocation in Claude Code requires a real session to verify the UX — automated tests cannot exercise the in-context skill execution path.
- **Generated specs lack `cmd` field**: Skills call `spec_create` which produces criteria with descriptions only (D076 from S03). Claude Code users who run `/assay:plan` still need to manually add `cmd` fields to their generated gates.toml before `assay gate run` is useful.
- **Stop hook scoping depends on PATH**: The `cycle-stop-check.sh` guard 5 (`assay` binary check) silently passes if `assay` is not on PATH — intended graceful degradation, but means the hook provides no value in unconfigured environments.

## Follow-ups

- S06 (Codex plugin): ports gate-check and spec-show from this plugin and adds cycle-status + plan skills
- Consider `spec_create` wizard enhancement that collects `cmd` per criterion (D076 revisit, future milestone)
- `milestone-checkpoint.sh` PreCompact hook mentioned in S05-PLAN.md vision but not required by Must-Haves — not implemented; deferred to future milestone

## Files Created/Modified

- `crates/assay-cli/src/commands/milestone.rs` — Added `json: bool` to `Status` variant; `milestone_status_cmd(json: bool)` with JSON branch; `milestone_status_json_no_active` test; updated `milestone_status_no_milestones` test
- `plugins/claude-code/skills/plan/SKILL.md` — new; interview-first workflow calling `milestone_create` + `spec_create`
- `plugins/claude-code/skills/status/SKILL.md` — new; `cycle_status` display with `{"active":false}` sentinel handling
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new; `cycle_status` + `chunk_status` + `spec_get` context loader
- `plugins/claude-code/CLAUDE.md` — replaced; 39-line reference with Skills, CLI, and MCP tables
- `plugins/claude-code/scripts/cycle-stop-check.sh` — new; cycle-aware Stop hook (made executable)
- `plugins/claude-code/scripts/post-tool-use.sh` — updated with active chunk name injection via `jq -n --arg`
- `plugins/claude-code/hooks/hooks.json` — Stop[0] command updated to `cycle-stop-check.sh`
- `plugins/claude-code/.claude-plugin/plugin.json` — version bumped to `0.5.0`
- `Cargo.toml` — workspace version bumped to `0.5.0`

## Forward Intelligence

### What the next slice should know
- The sentinel `{"active":false}` from `assay milestone status --json` lacks a `milestone_slug` key — detect active milestones via `jq 'has("milestone_slug")'`, not `.active` (D080). This same pattern applies to any S06 Codex skill that needs cycle state.
- All 8 MCP tools (milestone_list, milestone_get, cycle_status, cycle_advance, chunk_status, milestone_create, spec_create, pr_create) are tested and functional — S06 skills can call them directly without any Rust changes.
- The Claude Code skills in `plugins/claude-code/skills/` are the format template for S06 Codex skills: frontmatter (`name:` + `description:`) + `## Steps` + `## Output Format`.

### What's fragile
- `post-tool-use.sh` active chunk injection relies on `assay` being on PATH and `assay milestone status --json` succeeding — the hook uses `2>/dev/null` and checks exit code, so degradation is graceful, but the chunk name will be silently absent if assay is misconfigured.
- `cycle-stop-check.sh` guard 3 reads `MODE` from `~/.config/assay/stop-hook-mode` — this file must exist with value `enforce` to enable blocking behavior; missing file defaults to warn mode.

### Authoritative diagnostics
- `assay milestone status --json | jq .` — the canonical inspection command for what hooks will see; exits 0 always; stderr shows I/O errors
- `cat plugins/claude-code/scripts/cycle-stop-check.sh` — shows all 5 guards + cycle branch logic
- `cat plugins/claude-code/hooks/hooks.json` — confirms Stop[0] wiring

### What assumptions changed
- Workspace version bump was not anticipated in the plan — the `check-plugin-version` just recipe enforces plugin.json == Cargo.toml workspace version. Any future plugin version bump requires bumping both files.
