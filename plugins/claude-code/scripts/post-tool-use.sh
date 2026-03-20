#!/usr/bin/env bash
# PostToolUse reminder: nudge Claude to check gates when ready.
# This is reminder-only — it does NOT run gates.
# Fires after Write/Edit tool use.
#
# When an active milestone exists, names the current incomplete chunk in
# the reminder so the agent knows which context to check next.
# Gracefully degrades when `assay` is not on PATH or no .assay/ dir exists.

# Read stdin (tool input JSON) but we only need to output the reminder
cat > /dev/null

# Cycle detection: find first incomplete chunk slug
# Format: "  [ ] chunk-slug  (active)" — extract second field (the slug)
ACTIVE_CHUNK=$(assay milestone status 2>/dev/null | grep '\[ \]' | awk 'NR==1{print $2}')

if [ -n "$ACTIVE_CHUNK" ]; then
  MESSAGE="File modified. Active chunk: ${ACTIVE_CHUNK}. Run /assay:next-chunk to see active chunk context and gates."
else
  MESSAGE="File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass."
fi

jq -n --arg msg "$MESSAGE" '{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $msg
  }
}'

exit 0
