---
estimated_steps: 6
estimated_files: 4
---

# T02: Write cycle-stop-check.sh, update post-tool-use.sh, update hooks.json and plugin.json

**Slice:** S05 — Claude Code Plugin Upgrade
**Milestone:** M005

## Description

Write `cycle-stop-check.sh` — the cycle-aware replacement for `stop-gate-check.sh` — and update `post-tool-use.sh` to mention the active chunk in its reminder. Wire the new stop hook into `hooks.json` and bump `plugin.json` to version `0.5.0`.

`cycle-stop-check.sh` is the only moderately complex piece in S05. It must:
1. Preserve all 7 safety guards from `stop-gate-check.sh` (jq, stop_hook_active, MODE, .assay/ dir, binary, and the guard order matters for infinite-loop safety)
2. Add cycle-aware gate checking: parse `assay milestone status` output to find incomplete chunk slugs, run `gate run` per chunk, fall back to `gate run --all` when no active milestone exists
3. Produce the same block/warn/allow output format as the existing hook

The key risk documented in research is infinite-loop prevention: Guard 2 (`stop_hook_active`) must be the very first guard after Guard 1 (jq check). The existing `stop-gate-check.sh` is the authoritative reference implementation.

`post-tool-use.sh` is a simple update: detect the first active incomplete chunk slug and include it in the reminder text. The detection is cheap (disk read via `assay milestone status`) and fails gracefully when `assay` is not on PATH or no `.assay/` directory exists.

## Steps

1. Write `plugins/claude-code/scripts/cycle-stop-check.sh`. Structure:
   - Shebang + comment block naming the replaced script and explaining cycle-aware behavior
   - `INPUT=$(cat)` to read hook input JSON
   - **Guard 1**: `if ! command -v jq &>/dev/null; then exit 0; fi`
   - **Guard 2** (MUST be before all others after Guard 1): parse `stop_hook_active` from INPUT; `exit 0` if true — prevents infinite loops
   - **Guard 3**: `MODE="${ASSAY_STOP_HOOK_MODE:-enforce}"` check; `exit 0` if `off`
   - **Guard 4**: parse `CWD` from INPUT; `exit 0` if CWD empty or `.assay/` missing
   - **Guard 5**: `if ! command -v assay &>/dev/null; then exit 0; fi`
   - `cd "$CWD" || exit 0`
   - **Cycle detection**: `ACTIVE_CHUNKS=$(assay milestone status 2>/dev/null | grep '\[ \]' | awk '{print $2}')`
   - **Gate evaluation branch**: If `ACTIVE_CHUNKS` is non-empty, run `assay gate run "$chunk" --json` for each chunk and accumulate failures; if `ACTIVE_CHUNKS` is empty (no active milestone), fall back to `assay gate run --all --json`
   - If all gates pass (all chunk runs exit 0): `exit 0`
   - Count total failed criteria across all chunk runs
   - **Warn mode**: output `jq -n --arg count "$FAILED_COUNT" '{ systemMessage: "..." }'`; `exit 0`
   - **Enforce mode** (default): output `jq -n --arg count "$FAILED_COUNT" --arg chunks "$BLOCKING_CHUNKS" '{ decision: "block", reason: "..." }'`; include chunk slugs in the reason message so the agent knows which chunk to fix
   - Handle JSON parse failure of gate output (same as existing `stop-gate-check.sh` — output block with stderr snippet)

2. Update `plugins/claude-code/scripts/post-tool-use.sh`. Changes:
   - Keep the existing `cat > /dev/null` to discard stdin
   - Add cycle detection before the output: `ACTIVE_CHUNK=$(assay milestone status 2>/dev/null | grep '\[ \]' | awk 'NR==1{print $2}')`
   - Build the message: if `ACTIVE_CHUNK` is non-empty, use `"File modified. Active chunk: ${ACTIVE_CHUNK}. Run /assay:next-chunk to see active chunk context and gates."` otherwise use the existing message `"File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass."`
   - Replace the hardcoded `additionalContext` string with the computed `MESSAGE` variable using `jq -n --arg msg "$MESSAGE"` or a heredoc with variable interpolation
   - Graceful degradation: if `assay` is not on PATH, the `assay milestone status` call returns empty; the variable is empty; the existing message is used — no extra guard needed

3. Update `plugins/claude-code/hooks/hooks.json`:
   - In the `Stop` hooks array, change the first hook's `command` from `bash ${CLAUDE_PLUGIN_ROOT}/scripts/stop-gate-check.sh` to `bash ${CLAUDE_PLUGIN_ROOT}/scripts/cycle-stop-check.sh`
   - Leave the second Stop hook (`checkpoint-hook.sh`) unchanged
   - Leave all PostToolUse and PreCompact hooks unchanged

4. Bump `plugins/claude-code/.claude-plugin/plugin.json` version from `"0.4.0"` to `"0.5.0"`.

5. Verify scripts pass bash syntax check: `bash -n cycle-stop-check.sh` and `bash -n post-tool-use.sh`.

6. Run the full verification suite from the plan (bash -n, jq parse, grep checks).

## Must-Haves

- [ ] `plugins/claude-code/scripts/cycle-stop-check.sh` exists and `bash -n` passes
- [ ] Guard 2 (stop_hook_active) appears immediately after Guard 1 (jq check) — checked by line ordering
- [ ] `cycle-stop-check.sh` contains `assay milestone status` for cycle detection
- [ ] `cycle-stop-check.sh` falls back to `gate run --all` when `ACTIVE_CHUNKS` is empty
- [ ] `cycle-stop-check.sh` includes blocking chunk slugs in the `reason` field of block decisions
- [ ] `plugins/claude-code/scripts/post-tool-use.sh` is updated and `bash -n` passes
- [ ] `post-tool-use.sh` references `ACTIVE_CHUNK` and `/assay:next-chunk` in the cycle-aware message
- [ ] `plugins/claude-code/hooks/hooks.json` is valid JSON (jq parse succeeds)
- [ ] `hooks.json` references `cycle-stop-check.sh` in the Stop hook array
- [ ] `hooks.json` does NOT reference `stop-gate-check.sh` (0 matches)
- [ ] `plugins/claude-code/.claude-plugin/plugin.json` version is `"0.5.0"`

## Verification

```bash
cd plugins/claude-code

# Script syntax
bash -n scripts/cycle-stop-check.sh && echo "cycle-stop-check: OK"
bash -n scripts/post-tool-use.sh && echo "post-tool-use: OK"

# Guard 2 is immediately after Guard 1 (within first 20 lines of logic)
head -30 scripts/cycle-stop-check.sh | grep -n 'stop_hook_active\|jq'
# stop_hook_active line must come before line 25; jq guard before stop_hook_active

# Cycle detection present
grep 'milestone status' scripts/cycle-stop-check.sh

# Fallback to --all present
grep 'gate run --all' scripts/cycle-stop-check.sh

# Chunk slugs in reason message
grep 'BLOCKING_CHUNKS\|blocking\|chunk' scripts/cycle-stop-check.sh | grep -i 'reason\|block\|jq'

# Cycle-aware reminder in post-tool-use
grep 'ACTIVE_CHUNK\|next-chunk' scripts/post-tool-use.sh

# Valid JSON and correct hook reference
jq . hooks/hooks.json >/dev/null && echo "hooks.json: valid JSON"
grep 'cycle-stop-check' hooks/hooks.json && echo "new hook referenced"
grep -c 'stop-gate-check' hooks/hooks.json  # must be 0

# Version bump
grep '"0.5.0"' .claude-plugin/plugin.json && echo "version: OK"
```

## Observability Impact

- Signals added/changed: `cycle-stop-check.sh` adds chunk slug name to the `reason` field in block decisions — the agent now knows _which_ chunk to fix, not just that "N criteria are failing"
- How a future agent inspects this: the Stop hook output is surfaced by Claude Code as a block message; the agent reads `BLOCKING_CHUNKS` in the reason and can immediately call `/assay:gate-check <chunk-slug>` or `gate_run` MCP tool
- Failure state exposed: `ASSAY_STOP_HOOK_MODE` env var allows escalation from enforce→warn→off for debugging hook behavior without modifying the script

## Inputs

- `plugins/claude-code/scripts/stop-gate-check.sh` — authoritative reference for all 7 guards and the block/warn/allow output format; must be preserved verbatim in the new script except for the gate evaluation step
- `plugins/claude-code/scripts/post-tool-use.sh` — current file to be updated
- `plugins/claude-code/hooks/hooks.json` — current file; only one line changes (the command path in the first Stop hook)
- `plugins/claude-code/.claude-plugin/plugin.json` — current file; only the version field changes
- S05-RESEARCH.md — Cycle-Stop-Check Implementation Strategy section; PostToolUse Cycle-Aware Update section; Common Pitfalls (guard order, infinite-loop, null active_chunk_slug)
- `crates/assay-cli/src/commands/milestone.rs` — confirms `assay milestone status` prints `  [ ] chunk-slug  (active)` format (from S02 Forward Intelligence)

## Expected Output

- `plugins/claude-code/scripts/cycle-stop-check.sh` — new script: all 7 guards + cycle-aware gate checking + block/warn/allow output
- `plugins/claude-code/scripts/post-tool-use.sh` — updated: cycle-aware reminder message mentioning active chunk when present
- `plugins/claude-code/hooks/hooks.json` — updated: Stop hook references `cycle-stop-check.sh`
- `plugins/claude-code/.claude-plugin/plugin.json` — updated: version `"0.5.0"`
