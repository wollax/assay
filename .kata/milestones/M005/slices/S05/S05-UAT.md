# S05: Claude Code Plugin Upgrade — UAT

**Milestone:** M005
**Written:** 2026-03-20

## UAT Type

- UAT mode: mixed (artifact-driven for file/syntax checks; live-runtime for Claude Code session behavior)
- Why this mode is sufficient: Automated checks verify all static correctness (file existence, YAML frontmatter, bash syntax, JSON validity, line counts). Live runtime is required only to verify that skills render correctly inside Claude Code, that the Stop hook actually blocks, and that MCP tool calls succeed end-to-end. The automated tier covers ~80% of the surface; live session covers the remaining 20% of user-visible behavior.

## Preconditions

- Assay binary built and on PATH (`just build`)
- Claude Code installed with the Assay plugin loaded from `plugins/claude-code/`
- MCP server running with all M005 tools registered (`assay mcp`)
- At least one milestone exists in `.assay/milestones/` (or run `assay plan` to create one)
- `ASSAY_STOP_HOOK_MODE` unset or set to `enforce` for Stop hook testing

## Smoke Test

Open Claude Code, type `/assay:status`, confirm it displays either "No active milestone" or a milestone name/phase/chunk summary without errors.

## Test Cases

### 1. /assay:plan creates a milestone interactively

1. In Claude Code, invoke `/assay:plan`
2. Answer the goal prompt: "Add user authentication"
3. Specify 2 chunks: "database-schema" and "api-endpoints"
4. Provide 2 criteria each
5. Confirm the summary
6. **Expected:** `milestone_create` is called once; `spec_create` is called twice; milestone TOML appears in `.assay/milestones/`; gates.toml appears in `.assay/specs/` for each chunk

### 2. /assay:status shows active milestone

1. Ensure a milestone exists in `in_progress` state
2. Invoke `/assay:status`
3. **Expected:** Claude Code displays milestone name, phase (In Progress), active chunk slug, and progress count (e.g., "0/2 chunks complete")

### 3. /assay:status on empty project

1. Ensure no milestones exist in `.assay/milestones/`
2. Invoke `/assay:status`
3. **Expected:** Claude Code displays "No active milestone" and suggests `/assay:plan`

### 4. /assay:next-chunk shows active chunk context

1. Ensure an active milestone with an incomplete chunk
2. Invoke `/assay:next-chunk`
3. **Expected:** Claude Code displays the active chunk's slug, its criteria from `spec_get`, and gate pass/fail status from `chunk_status`

### 5. /assay:next-chunk on Verify phase (all chunks done)

1. Mark all chunks complete so the milestone is in Verify phase (`active_chunk_slug` is null)
2. Invoke `/assay:next-chunk`
3. **Expected:** Claude Code displays a message saying all chunks are complete and instructs the user to run `assay pr create`; does NOT crash or show an error

### 6. Stop hook blocks when active chunk has failing gates

1. Ensure an active milestone with an incomplete chunk that has failing required gates
2. Attempt to end the Claude Code session (Stop)
3. **Expected:** Stop hook fires; output contains `decision: "block"` with the failing chunk slug named in the reason field; session is blocked from ending

### 7. Stop hook falls back to --all when no active milestone

1. Ensure no active milestone exists
2. Attempt to end the Claude Code session
3. **Expected:** Stop hook fires `assay gate run --all --json`; if all gates pass, session ends normally; if gates fail, session is blocked

## Edge Cases

### /assay:plan abandonment mid-interview

1. Invoke `/assay:plan` but provide only the goal and then stop responding
2. **Expected:** No milestone or spec files are created (interview-first — tool calls happen only after full input collection and confirmation)

### Stop hook in warn mode

1. Set `ASSAY_STOP_HOOK_MODE=warn`
2. Ensure failing gates exist
3. End the session
4. **Expected:** Session ends normally; Claude Code shows a system message warning about failing gates and naming the blocking chunk slug; `decision` is not `"block"`

## Failure Signals

- `/assay:plan` calls `milestone_create` before the interview completes — skill is not interview-first (check SKILL.md step ordering)
- `/assay:next-chunk` returns an error instead of the "run assay pr create" message when `active_chunk_slug` is null — null guard is missing
- Stop hook fires but names no chunk slug — BLOCKING_CHUNKS was not populated or not included in the reason field
- `bash -n` fails on any script — syntax error in hook script
- `hooks.json` references `stop-gate-check.sh` — hooks.json was not updated

## Requirements Proved By This UAT

- R047 (Claude Code plugin upgrade) — live Claude Code session proves skill rendering, hook blocking, and MCP round-trips end-to-end; all 5 skills accessible; Stop hook names failing chunks; plugin.json at 0.5.0

## Not Proven By This UAT

- MCP server correctness for `milestone_create`, `cycle_status`, etc. — proven in S01–S04 integration tests
- Gate evaluation logic — proven in prior milestone gate tests
- Milestone file persistence — proven in S01 integration tests
- Actual PR creation — proven in S04 integration tests with mock `gh` binary
- S06 (Codex plugin) parity — separate UAT in S06-UAT.md

## Notes for Tester

- The Stop hook can be tested outside Claude Code: `echo '{}' | bash plugins/claude-code/scripts/cycle-stop-check.sh` (with stdin pipe simulating the Claude Code hook input format)
- ASSAY_STOP_HOOK_MODE=warn is useful for testing without blocking the session
- The interview-first constraint in `/assay:plan` is structural — look for whether `milestone_create` appears before or after the criteria collection steps in the skill output
- Plugin version 0.5.0 should appear in Claude Code plugin settings for the Assay plugin
