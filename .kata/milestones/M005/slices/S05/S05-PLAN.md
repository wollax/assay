# S05: Claude Code Plugin Upgrade

**Goal:** Upgrade the Claude Code plugin from a gate-runner integration to a full milestone-aware development cycle surface — new skills (`/assay:plan`, `/assay:status`, `/assay:next-chunk`), updated CLAUDE.md, a cycle-aware Stop hook, and an updated PostToolUse reminder that names the active chunk.
**Demo:** A Claude Code user can run `/assay:plan` to create a milestone, `/assay:status` to see cycle progress, and `/assay:next-chunk` to load the active chunk context. The Stop hook checks the active chunk's gates (falling back to `--all`) and blocks on failure in enforce mode, exactly like the existing hook but scoped to the active chunk when a milestone is in_progress.

## Must-Haves

- `assay milestone status --json` outputs `{"active":false}` when no milestone is in_progress; outputs `CycleStatus` JSON when one is active — exact shape match to the `cycle_status` MCP tool sentinel
- `plugins/claude-code/skills/plan/SKILL.md` — interviews user, calls `milestone_create` + `spec_create` per chunk
- `plugins/claude-code/skills/status/SKILL.md` — calls `cycle_status`, shows milestone/chunk/phase progress
- `plugins/claude-code/skills/next-chunk/SKILL.md` — calls `cycle_status` + `chunk_status` + `spec_get`, shows active chunk context
- `plugins/claude-code/CLAUDE.md` updated to document the full workflow (skills + CLI + MCP table)
- `plugins/claude-code/scripts/cycle-stop-check.sh` — all 5 safety guards from existing script verbatim; cycle-aware logic (scope to active chunk when in_progress, fallback to `--all` when not)
- `plugins/claude-code/scripts/post-tool-use.sh` updated to call `assay milestone status --json` and include active chunk name in reminder when available
- `plugins/claude-code/hooks/hooks.json` Stop[0] command updated from `stop-gate-check.sh` to `cycle-stop-check.sh`
- `plugins/claude-code/.claude-plugin/plugin.json` version bumped to `0.5.0`
- One new CLI test: `milestone_status_json_no_active` — exits 0, output is `{"active":false}`
- `just ready` green (1331+ tests pass)

## Proof Level

- This slice proves: integration (Rust CLI flag exercises existing `cycle_status` domain logic; bash hooks call `assay milestone status --json` via real subprocess)
- Real runtime required: no (hooks invoke the CLI which exercises `assay_core::milestone::cycle_status` — all paths covered by existing integration tests; bash hook logic verified by reading the script against guard semantics)
- Human/UAT required: yes — interactive skill invocation in Claude Code requires a real session to verify UX

## Verification

- `cargo test -p assay-cli -- milestone_status_json` — new test asserts exit 0 and `{"active":false}` output
- `cargo test --workspace` — 1331+ tests pass, 0 failures
- `just ready` — fmt + clippy + test + deny all green
- `bash -n plugins/claude-code/scripts/cycle-stop-check.sh` — no syntax errors
- `bash -n plugins/claude-code/scripts/post-tool-use.sh` — no syntax errors
- Content checks: `grep -l "milestone_create\|spec_create" plugins/claude-code/skills/plan/SKILL.md`, `grep "cycle_status" plugins/claude-code/skills/status/SKILL.md`, `grep "chunk_status" plugins/claude-code/skills/next-chunk/SKILL.md`
- `grep "0.5.0" plugins/claude-code/.claude-plugin/plugin.json`
- `grep "cycle-stop-check.sh" plugins/claude-code/hooks/hooks.json`

## Observability / Diagnostics

- Runtime signals: `assay milestone status --json` exits 0 always; stdout is the diagnostic surface
- Inspection surfaces: `assay milestone status --json | jq .` — shows active cycle state (or `{"active":false}`)
- Failure visibility: Stop hook outputs `{ decision: "block", reason: "..." }` JSON to stdout on gate failure in enforce mode; `{ systemMessage: "..." }` in warn mode — unchanged shape from existing hook
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `assay_core::milestone::cycle_status` (S02), `assay_core::milestone::CycleStatus` (S02), all 8 MCP tools registered in S01–S04 (`milestone_create`, `spec_create`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_list`, `milestone_get`, `pr_create`)
- New wiring introduced in this slice: `--json` flag on `MilestoneCommand::Status` → `assay_core::milestone::cycle_status` → serialized JSON output; hooks.json Stop[0] → `cycle-stop-check.sh` → `assay milestone status --json`
- What remains before the milestone is truly usable end-to-end: S06 (Codex plugin); R048

## Tasks

- [x] **T01: Add `--json` flag to `assay milestone status`** `est:30m`
  - Why: The cycle-stop-check.sh hook needs machine-readable cycle state from the CLI. The `--json` flag exposes `CycleStatus` JSON (or `{"active":false}`) using the existing `assay_core::milestone::cycle_status` function — no new domain logic required.
  - Files: `crates/assay-cli/src/commands/milestone.rs`
  - Do: Add `#[arg(long)] json: bool` to `MilestoneCommand::Status` variant. In `milestone_status_cmd`, add a `json: bool` parameter; when true, call `assay_core::milestone::cycle_status(&dir)` and serialize the result to stdout with `serde_json::to_string` — output `{"active":false}` on `Ok(None)`, the CycleStatus JSON on `Ok(Some(s))`, and exit 1 with eprintln on error (D072 pattern). Add a unit test `milestone_status_json_no_active` that sets up a tempdir with `.assay/` (no milestones), calls `handle(MilestoneCommand::Status { json: true })`, and asserts `result.is_ok()` and exit code 0. Run `just ready` after.
  - Verify: `cargo test -p assay-cli -- milestone_status_json` passes; `just ready` green
  - Done when: `cargo test -p assay-cli -- milestone_status_json` shows 1 passed, `just ready` exits 0

- [x] **T02: Write skill files and update CLAUDE.md** `est:30m`
  - Why: The three new skills are the primary user-facing surface of this slice — without them, Claude Code users have no workflow entry points for planning, status, and chunk context.
  - Files: `plugins/claude-code/skills/plan/SKILL.md`, `plugins/claude-code/skills/status/SKILL.md`, `plugins/claude-code/skills/next-chunk/SKILL.md`, `plugins/claude-code/CLAUDE.md`
  - Do:
    1. Create `plugins/claude-code/skills/plan/SKILL.md`: frontmatter (`name: plan`), Steps section — (1) interview user: ask milestone goal, chunk breakdown (how many chunks, slug+name+criteria per chunk), (2) call `milestone_create` MCP tool with the collected inputs, (3) call `spec_create` once per chunk with `milestone_slug` and criteria. Warn that generated gates have no `cmd` and require manual editing to be runnable. The interview step must precede MCP calls — never call `milestone_create` immediately on invocation (D066 intent).
    2. Create `plugins/claude-code/skills/status/SKILL.md`: frontmatter (`name: status`), Steps section — (1) call `cycle_status` MCP tool, (2) if `active == false`, report "No active milestone", (3) otherwise show milestone slug/name/phase, active chunk slug, completed/total counts with a progress bar (`[x][ ][ ]` style).
    3. Create `plugins/claude-code/skills/next-chunk/SKILL.md`: frontmatter (`name: next-chunk`), Steps section — (1) call `cycle_status` to find active milestone + active chunk slug, (2) if no active chunk, report "All chunks complete — run `assay milestone advance` or `assay pr create`", (3) call `chunk_status` with `chunk_slug` to show pass/fail summary, (4) call `spec_get` with the chunk slug to load full criteria, (5) present: chunk slug, criteria list (with pass/fail status from chunk_status), and the suggested next action.
    4. Replace `plugins/claude-code/CLAUDE.md` with an updated version: short intro paragraph, Skills table (`/assay:plan`, `/assay:status`, `/assay:next-chunk`, `/assay:spec-show`, `/assay:gate-check`), CLI table (`assay plan`, `assay milestone list`, `assay milestone status`, `assay milestone advance`, `assay pr create`), MCP tools table (all 8 new tools + existing `spec_list`, `spec_get`, `gate_run`). Keep concise — this is injected into every conversation.
  - Verify: `grep -l "milestone_create" plugins/claude-code/skills/plan/SKILL.md`, `grep "cycle_status" plugins/claude-code/skills/status/SKILL.md`, `grep "chunk_status" plugins/claude-code/skills/next-chunk/SKILL.md`, `grep "assay:plan" plugins/claude-code/CLAUDE.md`
  - Done when: all four files exist with the required content; `bash -n` syntax check is not needed (markdown); grep checks pass

- [x] **T03: Write cycle-stop-check.sh, update post-tool-use.sh, hooks.json, and plugin version** `est:30m`
  - Why: The hook infrastructure is what makes the plugin actively enforce cycle discipline — the cycle-stop-check closes the loop between skills and gate enforcement at conversation end.
  - Files: `plugins/claude-code/scripts/cycle-stop-check.sh`, `plugins/claude-code/scripts/post-tool-use.sh`, `plugins/claude-code/hooks/hooks.json`, `plugins/claude-code/.claude-plugin/plugin.json`
  - Do:
    1. Create `plugins/claude-code/scripts/cycle-stop-check.sh`: Copy guards 1–5 verbatim from `stop-gate-check.sh` (jq check, stop_hook_active, MODE, .assay/ dir, assay binary). Between guard 4 and guard 5, add cycle-aware logic: call `assay milestone status --json`, parse `.active` with `jq`. If active is false, fall through to existing `assay gate run --all --json` logic. If active is true, extract `active_chunk_slug` with `jq -r .active_chunk_slug`; if null or empty, fall back to `--all`; otherwise run `assay gate run "$ACTIVE_CHUNK_SLUG" --json`. The remainder of the script (failed count parse, warn/enforce mode logic, JSON output format) is identical to `stop-gate-check.sh`.
    2. Update `plugins/claude-code/scripts/post-tool-use.sh`: After consuming stdin, call `assay milestone status --json 2>/dev/null`; if exit 0 and `.active` is true, extract `active_chunk_slug` and include "Active chunk: <slug>" in the `additionalContext` message. If not active or assay is not found, keep the existing message unchanged. Must always exit 0 and never block.
    3. Update `plugins/claude-code/hooks/hooks.json`: Change `Stop[0].command` from `bash ${CLAUDE_PLUGIN_ROOT}/scripts/stop-gate-check.sh` to `bash ${CLAUDE_PLUGIN_ROOT}/scripts/cycle-stop-check.sh`. All other hooks unchanged.
    4. Bump `plugins/claude-code/.claude-plugin/plugin.json` version from `"0.4.0"` to `"0.5.0"`.
  - Verify: `bash -n plugins/claude-code/scripts/cycle-stop-check.sh`, `bash -n plugins/claude-code/scripts/post-tool-use.sh`, `grep "cycle-stop-check.sh" plugins/claude-code/hooks/hooks.json`, `grep "0.5.0" plugins/claude-code/.claude-plugin/plugin.json`; `just ready` green
  - Done when: All 4 files updated; `bash -n` passes on both scripts; `grep` checks confirm content; `just ready` exits 0

## Files Likely Touched

- `crates/assay-cli/src/commands/milestone.rs` — `--json` flag + test
- `plugins/claude-code/skills/plan/SKILL.md` — new
- `plugins/claude-code/skills/status/SKILL.md` — new
- `plugins/claude-code/skills/next-chunk/SKILL.md` — new
- `plugins/claude-code/CLAUDE.md` — updated
- `plugins/claude-code/scripts/cycle-stop-check.sh` — new
- `plugins/claude-code/scripts/post-tool-use.sh` — updated
- `plugins/claude-code/hooks/hooks.json` — Stop[0] command updated
- `plugins/claude-code/.claude-plugin/plugin.json` — version bump
