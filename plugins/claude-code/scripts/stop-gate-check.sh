#!/usr/bin/env bash
# Stop hook: verify quality gates pass before allowing agent to complete.
#
# Safety guards:
#   1. stop_hook_active = true -> allow stop (prevent infinite loops)
#   2. No .assay/ directory -> allow stop (graceful degradation)
#   3. assay binary not found -> allow stop (binary not installed)
#   4. ASSAY_STOP_HOOK_MODE=off -> allow stop (user disabled)
#   5. ASSAY_STOP_HOOK_MODE=warn -> allow stop with warning in reason
#
# Default: ASSAY_STOP_HOOK_MODE=enforce (block on failure)

INPUT=$(cat)

# Guard 1: Prevent infinite loops
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# Guard 4: Check if hook is disabled
MODE="${ASSAY_STOP_HOOK_MODE:-enforce}"
if [ "$MODE" = "off" ]; then
  exit 0
fi

# Guard 2: Graceful degradation — no .assay/ directory
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')
if [ -z "$CWD" ] || [ ! -d "$CWD/.assay" ]; then
  exit 0
fi

# Guard 3: Binary not installed
if ! command -v assay &>/dev/null; then
  exit 0
fi

# Run gate check for all specs
cd "$CWD" || exit 0
GATE_OUTPUT=$(assay gate run --all --json 2>&1)
GATE_EXIT=$?

# If gates pass (exit 0), allow stop
if [ $GATE_EXIT -eq 0 ]; then
  exit 0
fi

# Gates failed — action depends on mode
if [ "$MODE" = "warn" ]; then
  # Warn mode: allow stop but include warning
  exit 0
fi

# Enforce mode (default): block the stop
FAILED_COUNT=$(echo "$GATE_OUTPUT" | jq '[.[] | .failed] | add // 0' 2>/dev/null || echo "unknown")

cat <<EOF
{
  "decision": "block",
  "reason": "Quality gates are failing ($FAILED_COUNT criteria). Run /assay:gate-check for details and fix the failing criteria before completing."
}
EOF

exit 0
