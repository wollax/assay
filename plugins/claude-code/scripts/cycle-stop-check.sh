#!/usr/bin/env bash
# Stop hook: cycle-aware gate check — replaces stop-gate-check.sh.
#
# Extends the original 5-guard pattern with cycle-aware gate evaluation:
# instead of always running `gate run --all`, it discovers incomplete chunk
# slugs from `assay milestone status` and runs per-chunk gate checks.
# Falls back to `gate run --all` when no active milestone exists.
#
# Safety guards (order matters — especially Guard 2 before everything else):
#   1. jq not installed        -> allow stop (can't parse hook input)
#   2. stop_hook_active = true -> allow stop (MUST be first after jq — prevents infinite loops)
#   3. ASSAY_STOP_HOOK_MODE=off -> allow stop (user disabled)
#   4. No .assay/ directory    -> allow stop (graceful degradation)
#   5. assay binary not found  -> allow stop (binary not installed)
#
# Default: ASSAY_STOP_HOOK_MODE=enforce (block on failure)
# Override: ASSAY_STOP_HOOK_MODE=warn   (surface warning but allow stop)
#           ASSAY_STOP_HOOK_MODE=off    (disable entirely)

INPUT=$(cat)

# Guard 1: jq required for all subsequent JSON parsing
if ! command -v jq &>/dev/null; then
  exit 0
fi

# Guard 2: Prevent infinite loops — MUST immediately follow Guard 1
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# Guard 3: Check if hook is disabled
MODE="${ASSAY_STOP_HOOK_MODE:-enforce}"
if [ "$MODE" = "off" ]; then
  exit 0
fi

# Guard 4: Graceful degradation — no .assay/ directory
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')
if [ -z "$CWD" ] || [ ! -d "$CWD/.assay" ]; then
  exit 0
fi

# Guard 5: Binary not installed
if ! command -v assay &>/dev/null; then
  exit 0
fi

cd "$CWD" || exit 0

# Cycle detection: find incomplete chunk slugs from active milestone.
# Format: "  [ ] chunk-slug  (active)" — extract third field (the slug).
# Capture stderr to surface errors rather than silently falling back.
MILESTONE_STATUS_ERR=$(mktemp)
MILESTONE_STATUS_OUT=$(assay milestone status 2>"$MILESTONE_STATUS_ERR")
MILESTONE_STATUS_EXIT=$?
if [ "$MILESTONE_STATUS_EXIT" -ne 0 ]; then
  STDERR_CONTENT=$(cat "$MILESTONE_STATUS_ERR" | head -c 300)
  rm -f "$MILESTONE_STATUS_ERR"
  jq -n --arg err "$STDERR_CONTENT" '{
    decision: "block",
    reason: ("Could not determine active milestone (assay milestone status failed): " + $err + " Run `assay milestone status` manually to diagnose.")
  }'
  exit 0
fi
rm -f "$MILESTONE_STATUS_ERR"
ACTIVE_CHUNKS=$(echo "$MILESTONE_STATUS_OUT" | grep '\[ \]' | awk '{print $3}')

FAILED_COUNT=0
BLOCKING_CHUNKS=""
FALLBACK_MODE=false

if [ -n "$ACTIVE_CHUNKS" ]; then
  # Cycle-aware: run gate check per incomplete chunk
  while IFS= read -r chunk; do
    [ -z "$chunk" ] && continue
    GATE_STDERR=$(mktemp)
    GATE_OUTPUT=$(assay gate run "$chunk" --json 2>"$GATE_STDERR")
    GATE_EXIT=$?

    if [ "$GATE_EXIT" -ne 0 ]; then
      CHUNK_FAILED=$(echo "$GATE_OUTPUT" | jq '[.[] | .failed] | add // 0' 2>/dev/null)
      if ! [[ "$CHUNK_FAILED" =~ ^[0-9]+$ ]]; then
        # JSON parse failed — gate runner crashed or produced non-JSON output
        STDERR_CONTENT=$(cat "$GATE_STDERR" 2>/dev/null | head -c 300)
        rm -f "$GATE_STDERR"
        if [ -n "$STDERR_CONTENT" ]; then
          jq -n --arg chunk "$chunk" --arg err "$STDERR_CONTENT" '{
            decision: "block",
            reason: ("Gate check for chunk \($chunk) failed unexpectedly. Stderr: " + $err)
          }'
        else
          jq -n --arg chunk "$chunk" '{
            decision: "block",
            reason: ("Gate check for chunk \($chunk) failed unexpectedly. Run `assay gate run \($chunk)` manually to diagnose.")
          }'
        fi
        exit 0
      fi
      if [ "$CHUNK_FAILED" -eq 0 ]; then
        # Gate runner exited non-zero but reported 0 failures — tool error, not gate failure
        rm -f "$GATE_STDERR"
        jq -n --arg chunk "$chunk" '{
          decision: "block",
          reason: ("Gate check for chunk \($chunk) exited with an error but reported 0 failures. Run `assay gate run \($chunk)` manually to diagnose.")
        }'
        exit 0
      fi
      FAILED_COUNT=$((FAILED_COUNT + CHUNK_FAILED))
      if [ -n "$BLOCKING_CHUNKS" ]; then
        BLOCKING_CHUNKS="${BLOCKING_CHUNKS}, ${chunk}"
      else
        BLOCKING_CHUNKS="$chunk"
      fi
    fi
    rm -f "$GATE_STDERR"
  done <<< "$ACTIVE_CHUNKS"
else
  # No active milestone — fall back to gate run --all
  FALLBACK_MODE=true
  GATE_STDERR=$(mktemp)
  GATE_OUTPUT=$(assay gate run --all --json 2>"$GATE_STDERR")
  GATE_EXIT=$?

  if [ "$GATE_EXIT" -ne 0 ]; then
    FAILED_COUNT=$(echo "$GATE_OUTPUT" | jq '[.[] | .failed] | add // 0' 2>/dev/null)
    if ! [[ "$FAILED_COUNT" =~ ^[0-9]+$ ]]; then
      STDERR_CONTENT=$(cat "$GATE_STDERR" 2>/dev/null | head -c 500)
      rm -f "$GATE_STDERR"
      if [ -n "$STDERR_CONTENT" ]; then
        jq -n --arg err "$STDERR_CONTENT" '{
          decision: "block",
          reason: ("Gate check failed unexpectedly. Stderr: " + $err)
        }'
      else
        jq -n '{
          decision: "block",
          reason: "Gate check failed unexpectedly. Run `assay gate run --all` manually to diagnose."
        }'
      fi
      exit 0
    fi
  fi
  rm -f "$GATE_STDERR"
fi

# All gates pass
if [ "$FAILED_COUNT" -eq 0 ]; then
  exit 0
fi

# Build failure detail message — different for cycle-aware vs fallback paths
if [ "$FALLBACK_MODE" = "true" ]; then
  BLOCK_DETAIL="across all specs (no active milestone). Run \`assay gate run --all\` to review."
  WARN_DETAIL="across all specs (no active milestone)"
else
  FIRST_CHUNK=$(echo "$BLOCKING_CHUNKS" | cut -d',' -f1 | tr -d ' ')
  BLOCK_DETAIL="in chunks: ${BLOCKING_CHUNKS}. Run \`/assay:gate-check ${FIRST_CHUNK}\` to diagnose the first failing chunk, or \`/assay:next-chunk\` for active chunk context."
  WARN_DETAIL="in chunks: ${BLOCKING_CHUNKS}"
fi

# Gates failed — action depends on mode
if [ "$MODE" = "warn" ]; then
  jq -n --arg count "$FAILED_COUNT" --arg detail "$WARN_DETAIL" '{
    systemMessage: "Warning: quality gates are failing (\($count) criteria) \($detail). Fix before completing."
  }'
  exit 0
fi

# Enforce mode (default): block the stop, naming the blocking chunks
jq -n --arg count "$FAILED_COUNT" --arg detail "$BLOCK_DETAIL" '{
  decision: "block",
  reason: "Quality gates are failing (\($count) criteria) \($detail)"
}'

exit 0
