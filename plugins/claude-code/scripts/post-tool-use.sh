#!/usr/bin/env bash
# PostToolUse reminder: nudge Claude to check gates when ready.
# This is reminder-only — it does NOT run gates.
# Fires after Write/Edit tool use.

# Read stdin (tool input JSON) but we only need to output the reminder
cat > /dev/null

cat <<'EOF'
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass."
  }
}
EOF

exit 0
