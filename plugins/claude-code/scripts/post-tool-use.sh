#!/usr/bin/env bash
# PostToolUse reminder: nudge Claude to check gates when ready.
# This is reminder-only — it does NOT run gates.
# Fires after Write/Edit tool use.

# Read stdin (tool input JSON) but we only need to output the reminder
cat > /dev/null

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

jq -n --arg msg "File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass.${ACTIVE_CHUNK_MSG}" '{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $msg
  }
}'

exit 0
