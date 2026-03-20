# S05: Claude Code Plugin Upgrade — UAT

**Milestone:** M005
**Written:** 2026-03-20

## UAT Type

- UAT mode: mixed (artifact-driven + human-experience)
- Why this mode is sufficient: The Rust CLI change (`--json` flag) and bash hook logic are fully verified by automated tests and `bash -n` syntax checks. The human-experience component (interactive skill invocation in Claude Code) cannot be automated — it requires a real Claude Code session to verify the UX flow. Both modes are needed for complete coverage.

## Preconditions

For automated verification (already completed):
- `just ready` passes (1331+ tests, fmt, clippy, deny all green)
- `assay` binary available on PATH (`cargo build --release` or `cargo install --path crates/assay-cli`)

For human UAT:
- Claude Code installed and connected to an MCP server running Assay
- The Assay MCP server has `milestone_create`, `spec_create`, `cycle_status`, `chunk_status`, `spec_get`, `pr_create` tools registered
- A project with `.assay/` directory initialized (`assay init` run)
- `plugins/claude-code/` installed as a Claude Code plugin (`claude plugin install`)
- Plugin version 0.5.0 confirmed in Claude Code plugin list

## Smoke Test

Run `assay milestone status --json` in a project with no active milestone and confirm output is exactly `{"active":false}` and exit code is 0.

```bash
assay milestone status --json
echo "Exit: $?"
# Expected: {"active":false}\nExit: 0
```

## Test Cases

### 1. `--json` flag — no active milestone

```bash
cd /tmp/test-assay && mkdir -p .assay && assay milestone status --json
```

1. Run in a directory with `.assay/` but no milestone files
2. **Expected:** stdout is `{"active":false}`, exit code 0, stderr empty

### 2. `--json` flag — active milestone

1. Create a milestone TOML in `.assay/milestones/my-feature.toml` with `status = "in_progress"` and at least one chunk ref
2. Run `assay milestone status --json`
3. **Expected:** stdout is valid CycleStatus JSON with `milestone_slug`, `phase`, `active_chunk_slug`, `completed_count`, `total_count` fields; exit code 0

### 3. `/assay:plan` skill — happy path

1. In Claude Code, invoke `/assay:plan`
2. **Expected:** Claude asks for milestone goal (does NOT immediately call `milestone_create`)
3. Provide goal, chunk count (e.g. 2), and per-chunk slug/name/criteria when asked
4. **Expected:** Claude calls `milestone_create` with the collected inputs, then calls `spec_create` twice (once per chunk), reports success, and warns that generated gates have no `cmd` field

### 4. `/assay:status` skill — no active milestone

1. In Claude Code, invoke `/assay:status` with no active milestone in the project
2. **Expected:** Claude reports "No active milestone" (does not error or show partial data)

### 5. `/assay:status` skill — active milestone

1. Ensure a milestone is `in_progress` in `.assay/milestones/`
2. In Claude Code, invoke `/assay:status`
3. **Expected:** Claude shows milestone slug, name, phase, active chunk slug, and a progress display like `[x][ ][ ]` reflecting completed_count/total_count

### 6. `/assay:next-chunk` skill — active chunk with gate history

1. Ensure a milestone is `in_progress` with an active chunk that has gate run history
2. In Claude Code, invoke `/assay:next-chunk`
3. **Expected:** Claude calls `cycle_status`, then `chunk_status`, then `spec_get`; presents the active chunk slug, criteria list with ✓/✗ indicators, and a suggested next action

### 7. `/assay:next-chunk` skill — all chunks complete

1. Ensure a milestone is `in_progress` with `completed_count == total_count` (or `active_chunk_slug` is null)
2. In Claude Code, invoke `/assay:next-chunk`
3. **Expected:** Claude reports "All chunks complete — run `assay milestone advance` or `assay pr create`"

### 8. Stop hook — no active milestone (warn mode)

1. Ensure no active milestone; ensure `~/.config/assay/stop-hook-mode` does not contain `enforce`
2. End a Claude Code conversation (trigger Stop hook)
3. **Expected:** Hook runs `assay gate run --all --json` (fallback); outputs `{ systemMessage: "..." }` if gates fail or passes silently; exit 0

### 9. Stop hook — active milestone, active chunk (enforce mode)

1. Ensure a milestone is `in_progress` with a failing gate on the active chunk
2. Set `~/.config/assay/stop-hook-mode` to `enforce`
3. End a Claude Code conversation
4. **Expected:** Hook outputs `{ decision: "block", reason: "Quality gates failing for chunk '<slug>' (N criteria). Run /assay:gate-check <slug> for details." }`; Claude Code blocks the stop

### 10. PostToolUse — active chunk shown in reminder

1. Ensure a milestone is `in_progress` with an `active_chunk_slug`
2. Make a file edit in Claude Code
3. **Expected:** The PostToolUse hook fires and the reminder message includes "Active chunk: <slug>." appended to the additionalContext

## Edge Cases

### Missing `assay` binary (hook degradation)

1. Temporarily rename or remove `assay` from PATH
2. End a Claude Code conversation
3. **Expected:** `cycle-stop-check.sh` guard 5 triggers, exits 0 silently — conversation ends normally

### `--json` flag with I/O error

1. Create `.assay/milestones/bad.toml` with invalid TOML content
2. Run `assay milestone status --json`
3. **Expected:** stderr shows error message; exit code 1; stdout is empty (no partial JSON)

### PostToolUse when `assay` is not on PATH

1. Remove `assay` from PATH
2. Make a file edit in Claude Code
3. **Expected:** `post-tool-use.sh` completes without error (2>/dev/null suppresses assay error); reminder message uses the fallback text without chunk name; exit 0

## Failure Signals

- `/assay:plan` calls `milestone_create` immediately on invocation without interviewing the user → interview-first pattern broken
- `/assay:status` shows an error instead of "No active milestone" when no milestone exists
- `/assay:next-chunk` fails when `active_chunk_slug` is null instead of showing the "all chunks complete" message
- Stop hook exits non-zero when `assay` is not on PATH → conversation incorrectly blocked
- `post-tool-use.sh` exits non-zero (not 0) → PostToolUse hook breaks conversation flow
- `just ready` fails → regression introduced
- `grep "cycle-stop-check.sh" hooks.json` fails → old hook still wired

## Requirements Proved By This UAT

- R047 (Claude Code plugin upgrade) — UAT proves: (1) `--json` flag delivers machine-readable CycleStatus to hooks; (2) three new skills invoke the correct MCP tools with correct parameter shapes; (3) Stop hook correctly scopes gate evaluation to the active chunk; (4) PostToolUse reminder names the active chunk; (5) hooks.json wired to cycle-stop-check.sh; (6) plugin version 0.5.0

## Not Proven By This UAT

- Real `milestone_create` and `spec_create` MCP round-trips from within Claude Code skill execution (those MCP tools are tested independently in S03; skill invocation path requires human session)
- Stop hook behavior in a real git worktree with real failing gates (integration path requires live environment; hook logic is proven by bash syntax check + reading against guard semantics)
- Full end-to-end cycle from `/assay:plan` → work → `/assay:next-chunk` → gates pass → `/assay:status` → `assay pr create` — this is the M005 milestone-level UAT, not S05-specific
- `milestone-checkpoint.sh` PreCompact hook — not implemented in S05 (deferred)

## Notes for Tester

- The three skill files (`plan/SKILL.md`, `status/SKILL.md`, `next-chunk/SKILL.md`) must exist under `plugins/claude-code/skills/<name>/SKILL.md` — the directory name must match the frontmatter `name:` field exactly for Claude Code to register the skill command
- The `cycle-stop-check.sh` Stop hook replaces `stop-gate-check.sh` — the old script is still present but no longer wired in `hooks.json`. This is intentional.
- `assay milestone status --json` exits 0 in all non-I/O-error cases — this is by design (D081). A no-active-milestone response is not an error.
- Plugin version 0.5.0 requires workspace Cargo.toml version 0.5.0 — both files have been bumped; `just ready` confirms the match via `check-plugin-version`.
