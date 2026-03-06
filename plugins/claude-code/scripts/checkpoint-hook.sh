#!/usr/bin/env bash
# Checkpoint hook: save team state on task operations, compaction, or stop.
# Fires on PostToolUse[Task|TaskCreate|TaskUpdate], PreCompact, Stop.
# NEVER blocks agent workflow — always exits 0.

# Guard 1: jq required for JSON parsing
if ! command -v jq &>/dev/null; then
  exit 0
fi

# Guard 2: Read stdin
INPUT=$(cat)

# Guard 3: Prevent infinite loops on Stop events
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# Guard 4: CWD must exist and contain .assay/ directory
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')
if [ -z "$CWD" ] || [ ! -d "$CWD/.assay" ]; then
  exit 0
fi

# Guard 5: assay binary must be on PATH
if ! command -v assay &>/dev/null; then
  exit 0
fi

# Debounce: skip if last checkpoint was less than 5 seconds ago
DEBOUNCE_FILE="$CWD/.assay/checkpoints/.last-checkpoint-ts"
NOW=$(date +%s)
if [ -f "$DEBOUNCE_FILE" ]; then
  LAST_TS=$(cat "$DEBOUNCE_FILE" 2>/dev/null)
  if [[ "$LAST_TS" =~ ^[0-9]+$ ]]; then
    ELAPSED=$((NOW - LAST_TS))
    if [ "$ELAPSED" -lt 5 ]; then
      exit 0
    fi
  fi
fi

# Update debounce timestamp
mkdir -p "$CWD/.assay/checkpoints"
echo "$NOW" > "$DEBOUNCE_FILE"

# Extract trigger information
EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // "unknown"')
TOOL=$(echo "$INPUT" | jq -r '.tool_name // empty')
TRIGGER="${EVENT}${TOOL:+:$TOOL}"

# Extract session ID (may be empty)
SESSION=$(echo "$INPUT" | jq -r '.session_id // empty')

# Fire and forget: spawn checkpoint save in background
cd "$CWD" || exit 0
assay checkpoint save --trigger "$TRIGGER" ${SESSION:+--session "$SESSION"} &>/dev/null &

exit 0
