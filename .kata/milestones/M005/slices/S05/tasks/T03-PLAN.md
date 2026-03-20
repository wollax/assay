---
estimated_steps: 5
estimated_files: 4
---

# T03: Write cycle-stop-check.sh, update post-tool-use.sh, hooks.json, and plugin version

**Slice:** S05 — Claude Code Plugin Upgrade
**Milestone:** M005

## Description

Create `cycle-stop-check.sh` — a cycle-aware replacement for `stop-gate-check.sh` — that scopes gate evaluation to the active chunk when a milestone is in_progress. Update `post-tool-use.sh` to include the active chunk name in its PostToolUse reminder. Wire both changes into `hooks.json` and bump the plugin version to `0.5.0`.

T01 must be complete before this task because `cycle-stop-check.sh` calls `assay milestone status --json`.

## Steps

1. Create `plugins/claude-code/scripts/cycle-stop-check.sh`. Structure:
   - Copy guards 1–5 verbatim from `stop-gate-check.sh` (jq check, stop_hook_active, MODE env var, `.assay/` dir check, assay binary check). Do not alter their logic.
   - After guard 4 (`.assay/` check) and before guard 5 (binary check): add cycle-aware detection block:
     ```bash
     # Cycle-aware: detect active chunk for scoped gate evaluation
     CYCLE_JSON=$(assay milestone status --json 2>/dev/null)
     CYCLE_EXIT=$?
     ACTIVE_CHUNK=""
     if [ "$CYCLE_EXIT" -eq 0 ] && command -v jq &>/dev/null; then
       IS_ACTIVE=$(echo "$CYCLE_JSON" | jq -r '.active // (.milestone_slug // "false" | if . == "false" then "false" else "true" end)' 2>/dev/null)
       if [ "$IS_ACTIVE" != "false" ]; then
         ACTIVE_CHUNK=$(echo "$CYCLE_JSON" | jq -r '.active_chunk_slug // empty' 2>/dev/null)
       fi
     fi
     ```
     Note: `{"active":false}` does not have `.active` key explicitly — detect by checking if `milestone_slug` is absent. Simpler alternative: check `echo "$CYCLE_JSON" | jq 'has("milestone_slug")'`.
   - After guard 5 (binary check): use `$ACTIVE_CHUNK` to determine the gate run command:
     - If `$ACTIVE_CHUNK` is non-empty: `GATE_OUTPUT=$(assay gate run "$ACTIVE_CHUNK" --json 2>"$GATE_STDERR")`
     - Otherwise (no active milestone or all chunks complete): `GATE_OUTPUT=$(assay gate run --all --json 2>"$GATE_STDERR")`
   - The remainder (failed count parsing, warn/enforce mode, JSON output) is identical to `stop-gate-check.sh`.
   - Make the file executable: `chmod +x plugins/claude-code/scripts/cycle-stop-check.sh`

2. Update `plugins/claude-code/scripts/post-tool-use.sh`:
   - After `cat > /dev/null` (consume stdin), add:
     ```bash
     ACTIVE_CHUNK_MSG=""
     if command -v assay &>/dev/null; then
       CYCLE_JSON=$(assay milestone status --json 2>/dev/null)
       if [ $? -eq 0 ] && echo "$CYCLE_JSON" | jq -e 'has("milestone_slug")' &>/dev/null; then
         CHUNK=$(echo "$CYCLE_JSON" | jq -r '.active_chunk_slug // empty' 2>/dev/null)
         if [ -n "$CHUNK" ]; then
           ACTIVE_CHUNK_MSG=" Active chunk: $CHUNK."
         fi
       fi
     fi
     ```
   - In the `additionalContext` string, append `$ACTIVE_CHUNK_MSG` after the existing message: `"File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass.${ACTIVE_CHUNK_MSG}"`
   - Must always exit 0. If assay is not on PATH or milestone status fails, `$ACTIVE_CHUNK_MSG` stays empty and the existing message is unchanged.

3. Update `plugins/claude-code/hooks/hooks.json`: change the `Stop[0]` hook command from `bash ${CLAUDE_PLUGIN_ROOT}/scripts/stop-gate-check.sh` to `bash ${CLAUDE_PLUGIN_ROOT}/scripts/cycle-stop-check.sh`. All other hooks (PostToolUse, PreCompact, Stop[1]) unchanged.

4. Bump `plugins/claude-code/.claude-plugin/plugin.json` version from `"0.4.0"` to `"0.5.0"`.

5. Run `just ready` to confirm no regressions.

## Must-Haves

- [ ] `cycle-stop-check.sh` contains all 5 guards from `stop-gate-check.sh` verbatim before the gate run command
- [ ] When `assay milestone status --json` reports an active milestone with a non-null `active_chunk_slug`, the hook runs `assay gate run "$ACTIVE_CHUNK_SLUG" --json`
- [ ] When no milestone is active or `active_chunk_slug` is null, the hook falls back to `assay gate run --all --json`
- [ ] `post-tool-use.sh` always exits 0; `$ACTIVE_CHUNK_MSG` is empty when assay is not found or no active chunk
- [ ] `hooks.json` Stop[0] points to `cycle-stop-check.sh`
- [ ] `plugin.json` version is `"0.5.0"`
- [ ] `just ready` exits 0

## Verification

- `bash -n plugins/claude-code/scripts/cycle-stop-check.sh` — no syntax errors
- `bash -n plugins/claude-code/scripts/post-tool-use.sh` — no syntax errors
- `grep "cycle-stop-check.sh" plugins/claude-code/hooks/hooks.json` — exits 0
- `grep "stop-gate-check.sh" plugins/claude-code/hooks/hooks.json` — should exit non-zero (old reference removed from Stop[0])
- `grep '"version": "0.5.0"' plugins/claude-code/.claude-plugin/plugin.json` — exits 0
- `just ready` — "All checks passed."

## Observability Impact

- Signals added/changed: Stop hook now surfaces active chunk slug in block reason message when scoping to active chunk (e.g. "Quality gates failing for chunk 'my-feature' (2 criteria). Run /assay:gate-check my-feature for details.")
- How a future agent inspects this: `cat plugins/claude-code/scripts/cycle-stop-check.sh` — shows guard logic; `assay milestone status --json | jq .` — confirms what the hook will see at runtime
- Failure state exposed: `ASSAY_STOP_HOOK_MODE=warn` env var degrades to systemMessage instead of blocking — useful when gate suite is slow

## Inputs

- `plugins/claude-code/scripts/stop-gate-check.sh` — copy guards 1–5 verbatim; copy warn/enforce output logic verbatim
- `plugins/claude-code/scripts/post-tool-use.sh` — existing file to update
- `plugins/claude-code/hooks/hooks.json` — existing file to update
- `plugins/claude-code/.claude-plugin/plugin.json` — version field to bump
- T01-PLAN.md / T01 output — `assay milestone status --json` flag must be complete for cycle-stop-check.sh to work
- S05-RESEARCH.md Common Pitfalls — `{"active":false}` detection via `jq 'has("milestone_slug")'`; `active_chunk_slug` can be null

## Expected Output

- `plugins/claude-code/scripts/cycle-stop-check.sh` — new cycle-aware stop hook with all guards
- `plugins/claude-code/scripts/post-tool-use.sh` — updated with active chunk name injection
- `plugins/claude-code/hooks/hooks.json` — Stop[0] command updated
- `plugins/claude-code/.claude-plugin/plugin.json` — version `0.5.0`
