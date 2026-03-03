#!/usr/bin/env bash
# Stop hook: verify quality gates pass before allowing agent to complete.
#
# Safety guards:
#   1. jq not installed -> allow stop (can't parse hook input)
#   2. stop_hook_active = true -> allow stop (prevent infinite loops)
#   3. ASSAY_STOP_HOOK_MODE=off -> allow stop (user disabled)
#   4. No .assay/ directory -> allow stop (graceful degradation)
#   5. assay binary not found -> allow stop (binary not installed)
#
# Default: ASSAY_STOP_HOOK_MODE=enforce (block on failure)

INPUT=$(cat)

# Guard 1: jq required for all subsequent JSON parsing
if ! command -v jq &>/dev/null; then
  exit 0
fi

# Guard 2: Prevent infinite loops
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

# Run gate check for all specs
cd "$CWD" || exit 0
GATE_STDERR=$(mktemp)
GATE_OUTPUT=$(assay gate run --all --json 2>"$GATE_STDERR")
GATE_EXIT=$?

# If gates pass (exit 0), allow stop
if [ "$GATE_EXIT" -eq 0 ]; then
  exit 0
fi

# Extract failed count for diagnostics
FAILED_COUNT=$(echo "$GATE_OUTPUT" | jq '[.[] | .failed] | add // 0' 2>/dev/null)
if ! [[ "$FAILED_COUNT" =~ ^[0-9]+$ ]]; then
  # JSON parse failed — include stderr for diagnosis
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
rm -f "$GATE_STDERR"

# Gates failed — action depends on mode
if [ "$MODE" = "warn" ]; then
  # Warn mode: allow stop but surface warning via systemMessage
  jq -n --arg count "$FAILED_COUNT" '{
    systemMessage: "Warning: quality gates are failing (\($count) criteria). Run /assay:gate-check to review."
  }'
  exit 0
fi

# Enforce mode (default): block the stop
jq -n --arg count "$FAILED_COUNT" '{
  decision: "block",
  reason: "Quality gates are failing (\($count) criteria). Run /assay:gate-check for details and fix the failing criteria before completing."
}'

exit 0
