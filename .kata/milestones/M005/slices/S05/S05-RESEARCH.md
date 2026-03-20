# S05: Claude Code Plugin Upgrade — Research

**Date:** 2026-03-20
**Domain:** Claude Code plugin authoring (markdown skills, bash hooks)
**Confidence:** HIGH

## Summary

S05 is a pure-markdown and bash slice — no Rust changes. All 8 new MCP tools (milestone_list, milestone_get, milestone_create, spec_create, cycle_status, cycle_advance, chunk_status, pr_create) are already registered and tested. The plugin needs 3 new skills, an updated CLAUDE.md, a replacement Stop hook (`cycle-stop-check.sh`), and a cycle-aware PostToolUse nudge.

The existing plugin structure is clean and well-precedented. The key implementation challenge is the `cycle-stop-check.sh`: it must detect the active chunk without a JSON-outputting CLI command, then run targeted gate checks. The recommended approach is to parse `assay milestone status` human-readable output (which is stable and already tested) to find incomplete chunks.

The `/assay:plan` skill must follow the interview-first pattern (collect goal → chunks → criteria conversationally before calling `milestone_create`+`spec_create`) — it must never call MCP tools on invocation without first gathering user input. This is the most UX-sensitive part of the slice.

## Recommendation

Implement skills and hooks in this order: (1) `plan.md` and `status.md` (simplest contracts), (2) `next-chunk.md` (most complex — three MCP calls), (3) updated CLAUDE.md, (4) `cycle-stop-check.sh`, (5) updated `post-tool-use.sh`, (6) updated `hooks.json`. Test the stop hook locally — the bash parsing of `assay milestone status` output is the most fragile piece.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Detecting active chunk in bash | `assay milestone status \| grep '\[ \]' \| awk '{print $2}'` | milestone status prints `[ ] chunk-slug  (active)` lines — parseable without JSON flag or new Rust code |
| Gate enforcement in Stop hook | Reuse the 7-guard pattern from existing `stop-gate-check.sh` | jq check, infinite-loop guard, mode env var, `.assay/` guard, binary guard, debounce — all proven patterns |
| Checkpoint on PreCompact/Stop | Existing `checkpoint-hook.sh` unchanged | The milestone TOML is already atomically persisted by every `cycle_advance` call — no extra checkpoint needed |
| Hook output format | `jq -n '{decision: "block", reason: "..."}'` | Claude Code hook protocol: output `decision`/`reason` JSON to block; `systemMessage` JSON to warn |

## Existing Code and Patterns

- `plugins/claude-code/skills/gate-check/SKILL.md` — template for skill file structure: YAML frontmatter + `description` + `## Steps` (numbered) + `## Output Format` (concise rules); reuse exactly
- `plugins/claude-code/skills/spec-show/SKILL.md` — same template; note `$ARGUMENTS` placeholder for skill invocation arguments
- `plugins/claude-code/scripts/stop-gate-check.sh` — 7-guard pattern (jq, stop_hook_active, MODE env, .assay/ dir, binary, JSON parse, mode dispatch); the new `cycle-stop-check.sh` must implement all 7 guards
- `plugins/claude-code/scripts/checkpoint-hook.sh` — 5-guard pattern + 5-second debounce via `.assay/checkpoints/.last-checkpoint-ts`; do NOT modify — already wired to Stop and PreCompact
- `plugins/claude-code/scripts/post-tool-use.sh` — outputs `hookSpecificOutput.additionalContext` JSON; minimal change needed (add cycle context when active)
- `plugins/claude-code/hooks/hooks.json` — hook event keys: `PostToolUse` (matcher required), `PreCompact`, `Stop`; `type: "command"`, `timeout` in seconds; `bash ${CLAUDE_PLUGIN_ROOT}/scripts/<name>.sh`
- `plugins/claude-code/.claude-plugin/plugin.json` — plugin metadata; `version` should be bumped to `0.5.0` for the M005 upgrade
- `plugins/claude-code/.mcp.json` — no changes needed; `assay mcp serve` already exposes all 30 tools

## MCP Tool Contracts (what skills call)

### `cycle_status` (no params)
Returns one of:
- `{"active":false}` — no InProgress milestone
- `{"milestone_slug":"...", "milestone_name":"...", "phase":"in_progress"|"verify"|"draft"|"complete", "active_chunk_slug":"..."|null, "completed_count":N, "total_count":N}`

### `chunk_status` (params: `chunk_slug: String`)
Returns one of:
- `{"chunk_slug":"...", "has_history":false}` — no gate history yet
- `{"chunk_slug":"...", "has_history":true, "latest_run_id":"...", "passed":N, "failed":N, "required_failed":N}`

### `milestone_create` (params: `slug, name, description?, chunks: [{slug, name, criteria: [strings]}]`)
Returns: JSON-encoded slug string on success; `isError:true` with collision message on duplicate.

### `spec_create` (params: `slug, name, description?, milestone_slug?, criteria: [strings]`)
Returns: absolute path to created `gates.toml` on success; `isError:true` on duplicate or missing milestone.

### `pr_create` (params: `milestone_slug, title, body?`)
Returns: `{"pr_number":N, "pr_url":"..."}` on success; `isError:true` with failing chunk list on gate failure.

### `spec_get` (params: `name`)
Returns: full spec definition including all criteria — used by `next-chunk.md` to show criteria.

## Skill Directory Structure

Skills for Claude Code live in **subdirectories** (not flat files):
```
plugins/claude-code/skills/
  gate-check/SKILL.md    ← existing
  spec-show/SKILL.md     ← existing
  plan/SKILL.md          ← new
  status/SKILL.md        ← new
  next-chunk/SKILL.md    ← new
```

(Codex skills in S06 are flat `.md` files — the convention differs by platform.)

## Cycle-Stop-Check Implementation Strategy

The `cycle-stop-check.sh` replaces `stop-gate-check.sh` in `hooks.json`. The approach:

1. All 7 guards from `stop-gate-check.sh` are preserved.
2. Detect active chunk via:
   ```bash
   ACTIVE_CHUNKS=$(assay milestone status 2>/dev/null | grep '\[ \]' | awk '{print $2}')
   ```
   The `assay milestone status` output format is stable: `  [ ] chunk-slug  (active)`.
3. If `ACTIVE_CHUNKS` is non-empty: run `assay gate run "$chunk" --json` for each incomplete chunk, aggregate results.
4. If no active milestone (empty output): fall back to `assay gate run --all --json` — preserves backward compat for non-milestone projects.
5. Block/warn/allow based on `ASSAY_STOP_HOOK_MODE` env var (same as existing).

**Key risk**: `assay milestone status` makes a subprocess call (disk I/O). This is fine for a Stop hook (user-facing, blocking is acceptable) but not for PostToolUse (fires on every Write/Edit). PostToolUse should use a cheaper detection.

## PostToolUse Cycle-Aware Update

The updated `post-tool-use.sh` adds cycle awareness to the reminder message:

```bash
ACTIVE_CHUNK=$(assay milestone status 2>/dev/null | grep '\[ \]' | awk 'NR==1{print $2}')
if [ -n "$ACTIVE_CHUNK" ]; then
  MESSAGE="File modified. Active chunk: ${ACTIVE_CHUNK}. Run /assay:next-chunk to see active chunk context and gates."
else
  MESSAGE="File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass."
fi
```

This call is cheap (disk read, no gate evaluation). The existing 5-guard structure is preserved.

## Constraints

- **No Rust changes** — all MCP tools already exist; `assay milestone status` output parsing is the entire implementation surface for hook intelligence
- **No `.mcp.json` changes** — `assay mcp serve` exposes all 30 tools; no new server configuration
- **Plan skill is interview-first** — never call `milestone_create` without collecting `goal`, `chunk breakdown`, and `criteria` from the user first; the MCP tools accept user-provided slugs, not auto-derived ones
- **CLAUDE.md stays ≤50 lines** — it's injected into every conversation; keep to a skills table + CLI table + MCP tools table + 1-paragraph workflow; detailed instructions live in the skills themselves
- **`next-chunk.md` calls three tools** — `cycle_status` → `chunk_status` → `spec_get` in sequence; if `cycle_status` returns `{"active":false}` the skill stops and tells the user no active chunk
- **Hook timeout budget** — Stop hook has `timeout: 120s` (from existing config); `cycle-stop-check.sh` may run `assay gate run` per-chunk, which can be slow; consider keeping the `--all` fallback but adding the cycle-aware block message
- **`checkpoint-hook.sh` is unchanged** — it fires on `Stop` after `cycle-stop-check.sh` in the hooks.json array; checkpoint saves session state regardless of gate outcome

## Common Pitfalls

- **skills/plan/SKILL.md calling MCP tools immediately** — plan skill must open with a conversational interview ("Tell me about the feature you want to build...") before calling `milestone_create`; if it calls the MCP tool immediately, users lose the wizard UX that makes Assay accessible
- **stop hook calling itself** — always keep Guard 2 (`stop_hook_active` check) as the first guard after jq-check; infinite loops cause Claude Code to hang
- **chunk slug extraction from milestone status** — `awk '{print $2}'` works when status output is `  [ ] chunk-slug  (active)`; test that the chunk slug contains no spaces (slugs are path components so this is guaranteed)
- **next-chunk.md handling null active_chunk_slug** — `cycle_status` returns `active_chunk_slug: null` when all chunks are done (milestone in Verify phase); the skill must handle this case gracefully (tell user to run `assay pr create`)
- **CLAUDE.md referencing old hook script name** — if CLAUDE.md mentions `stop-gate-check.sh`, update it to reference the new script or remove the implementation detail

## Open Risks

- `assay milestone status` output format — the `[ ] chunk-slug  (active)` format is produced by `milestone_status_cmd()` in `assay-cli/src/commands/milestone.rs`; if this format changes in a future slice, the bash parsing breaks. Acceptable risk for a low-risk slice (risk:low per roadmap).
- Multiple InProgress milestones — `assay milestone status` prints all InProgress milestones; `grep '\[ \]' | awk '{print $2}'` will return chunks from all of them. The cycle-stop-check would then run gates for all active chunks across all active milestones — likely the desired behavior.
- Skills calling `cycle_advance` — the current scope says skills *reference* `cycle_advance` in CLAUDE.md but don't call it directly; agents call it themselves when ready. This is a scope boundary, not an implementation risk.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Claude Code plugin authoring | N/A — markdown + bash, no external skill needed | N/A |
| Bash hooks | N/A — patterns fully covered by existing `stop-gate-check.sh` | N/A |

No external skills are relevant — this slice is pure plugin authoring using established patterns.

## Sources

- `plugins/claude-code/` — full existing plugin structure inspected (hooks.json, all scripts, SKILL.md files, CLAUDE.md, plugin.json)
- `crates/assay-mcp/src/server.rs` — MCP tool param structs and async handlers; confirmed all 8 new tools registered (30 tools total: lines 1105–3646)
- `crates/assay-cli/src/commands/milestone.rs` — `milestone_status_cmd()` output format; confirmed `[ ] chunk-slug  (active)` format used by stop hook strategy
- `crates/assay-core/src/milestone/cycle.rs` — `CycleStatus` struct; confirmed `active_chunk_slug: Option<String>` and `{"active":false}` null-path return
- `.kata/milestones/M005/slices/S05/S05-CONTEXT.md` — scope decisions, confirmed `milestone-checkpoint.sh` dropped, PostToolUse subprocess approach approved
- S01–S04 summaries — confirmed all MCP tools exist, param shapes, and forward intelligence for consumers
