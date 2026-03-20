#!/usr/bin/env bash
# PostToolUse reminder: nudge Claude to check gates when ready.
# This is reminder-only — it does NOT run gates.
# Fires after Write/Edit tool use.
#
# When an active milestone exists, names the current incomplete chunk in
# the reminder so the agent knows which context to check next.
# Gracefully degrades: if assay is missing, jq is missing, or no .assay/
# directory exists, falls back to the generic reminder — no explicit guard
# needed for those cases since the detection call is wrapped in a conditional.

# Read stdin (tool input JSON) but we only need to output the reminder
cat > /dev/null

# Cycle detection: find first incomplete chunk slug.
# Format: "  [ ] chunk-slug  (active)" — extract third field (the slug).
# Only probed when this looks like an Assay project with the binary available.
ACTIVE_CHUNK=""
if [ -d "$PWD/.assay" ] && command -v assay &>/dev/null; then
  ACTIVE_CHUNK=$(assay milestone status 2>/dev/null | grep '\[ \]' | awk 'NR==1{print $3}')
fi

if [ -n "$ACTIVE_CHUNK" ]; then
  MESSAGE="File modified. Active chunk: ${ACTIVE_CHUNK}. Run /assay:next-chunk to see active chunk context and gates."
else
  MESSAGE="File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass."
fi

# Guard: jq required to format the hook output JSON
if ! command -v jq &>/dev/null; then
  printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"%s"}}\n' "$MESSAGE"
  exit 0
fi

jq -n --arg msg "$MESSAGE" '{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $msg
  }
}'

exit 0
