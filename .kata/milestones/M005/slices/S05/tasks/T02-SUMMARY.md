---
id: T02
parent: S05
milestone: M005
provides:
  - plugins/claude-code/scripts/cycle-stop-check.sh — cycle-aware Stop hook with 7 safety guards and per-chunk gate evaluation
  - plugins/claude-code/scripts/post-tool-use.sh — updated to surface active chunk name in reminder text
  - plugins/claude-code/hooks/hooks.json — Stop hook wired to cycle-stop-check.sh
  - plugins/claude-code/.claude-plugin/plugin.json — version bumped to 0.5.0
key_files:
  - plugins/claude-code/scripts/cycle-stop-check.sh
  - plugins/claude-code/scripts/post-tool-use.sh
  - plugins/claude-code/hooks/hooks.json
  - plugins/claude-code/.claude-plugin/plugin.json
key_decisions:
  - Guard 2 (stop_hook_active) is enforced at line 28 — immediately after Guard 1 (jq check at line 23) — preventing infinite loops before any other logic runs
  - cycle-stop-check.sh parses `assay milestone status` for `[ ]` lines and runs `gate run $chunk --json` per incomplete chunk; falls back to `gate run --all --json` when no active chunks exist
  - BLOCKING_CHUNKS variable accumulates failing chunk slugs across per-chunk runs; included verbatim in the `reason` field of block decisions so the agent knows which chunks to fix
  - post-tool-use.sh uses `assay milestone status` detection before output; graceful degradation is implicit (empty ACTIVE_CHUNK → fallback message) with no extra guard needed
  - JSON parse failure per-chunk produces an immediate block with stderr snippet rather than continuing to other chunks
patterns_established:
  - cycle-aware stop hook pattern: discover incomplete chunks → run per-chunk gate checks → accumulate BLOCKING_CHUNKS → name them in block reason
  - guard-order pattern: jq (Guard 1) → stop_hook_active (Guard 2) → MODE (Guard 3) → .assay/ dir (Guard 4) → binary (Guard 5) → cd → work
observability_surfaces:
  - Stop hook block output: `{ decision: "block", reason: "... in chunks: chunk-slug-a, chunk-slug-b ..." }` — agent reads BLOCKING_CHUNKS in reason to target `/assay:gate-check <slug>`
  - Warn mode: `{ systemMessage: "Warning: ... in chunks: ..." }` via ASSAY_STOP_HOOK_MODE=warn
  - PostToolUse: `additionalContext` now names the active chunk slug when present
  - ASSAY_STOP_HOOK_MODE env var: enforce (default) | warn | off — controls hook escalation without script modification
duration: 20min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T02: Write cycle-stop-check.sh, update post-tool-use.sh, update hooks.json and plugin.json

**Cycle-aware Stop hook (`cycle-stop-check.sh`) replaces `stop-gate-check.sh`, naming failing chunk slugs in block decisions; PostToolUse reminder updated to surface the active chunk; hooks.json and plugin.json wired and bumped to 0.5.0.**

## What Happened

Wrote `cycle-stop-check.sh` by extending the existing 7-guard pattern from `stop-gate-check.sh`. The core addition is cycle detection via `assay milestone status 2>/dev/null | grep '\[ \]' | awk '{print $2}'` which extracts incomplete chunk slugs. When chunks are found, the script runs `gate run "$chunk" --json` for each and accumulates failures into `FAILED_COUNT` and `BLOCKING_CHUNKS`. When no active milestone exists (empty ACTIVE_CHUNKS), it falls back to `gate run --all --json` matching the original behavior.

The block/warn output in both branches now includes `$BLOCKING_CHUNKS` in the reason/systemMessage, so the agent immediately knows which chunk to fix without a separate diagnostic call.

Updated `post-tool-use.sh` to detect the first incomplete chunk with `awk 'NR==1{print $2}'` and embed it in the `additionalContext` message. If `assay` is absent or returns nothing, `ACTIVE_CHUNK` is empty and the original message is used — no extra guard needed.

Updated `hooks.json` to reference `cycle-stop-check.sh` (one line change in the first Stop hook). Bumped `plugin.json` from `0.4.0` to `0.5.0`.

## Verification

All must-haves verified:

```
bash -n scripts/cycle-stop-check.sh    → OK
bash -n scripts/post-tool-use.sh       → OK

Guard ordering: jq at line 23, stop_hook_active at line 28 (Guard 2 immediately follows Guard 1)

grep 'milestone status' scripts/cycle-stop-check.sh  → present (2 matches: comment + logic)
grep 'gate run --all'   scripts/cycle-stop-check.sh  → present (fallback branch)
BLOCKING_CHUNKS in jq reason output                  → confirmed in both warn+enforce paths

grep 'ACTIVE_CHUNK\|next-chunk' scripts/post-tool-use.sh  → present

jq . hooks/hooks.json       → valid JSON
grep 'cycle-stop-check' hooks/hooks.json   → referenced
grep -c 'stop-gate-check' hooks/hooks.json → 0

grep '"0.5.0"' plugin.json  → version: OK

grep -c 'exit 0' scripts/cycle-stop-check.sh  → 11 (≥7 safety guard exits)
```

## Diagnostics

- Run `ASSAY_STOP_HOOK_MODE=warn bash plugins/claude-code/scripts/cycle-stop-check.sh <<< '{}'` to test warn-mode output
- Run `ASSAY_STOP_HOOK_MODE=off bash ...` to disable the hook for debugging
- The block reason names `BLOCKING_CHUNKS` — read it to identify which chunk slug to pass to `/assay:gate-check <slug>`
- `assay milestone status` confirms which chunks are incomplete at any time

## Deviations

None. Implemented exactly per the task plan.

## Known Issues

None.

## Files Created/Modified

- `plugins/claude-code/scripts/cycle-stop-check.sh` — new: 7-guard cycle-aware Stop hook with per-chunk gate evaluation and BLOCKING_CHUNKS in reason output
- `plugins/claude-code/scripts/post-tool-use.sh` — updated: cycle-aware additionalContext mentioning active chunk slug when present
- `plugins/claude-code/hooks/hooks.json` — updated: Stop hook command points to cycle-stop-check.sh
- `plugins/claude-code/.claude-plugin/plugin.json` — updated: version 0.4.0 → 0.5.0
