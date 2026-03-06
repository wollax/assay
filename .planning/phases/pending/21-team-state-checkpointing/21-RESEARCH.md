# Phase 21: Team State Checkpointing - Research

**Completed:** 2026-03-06
**Confidence:** HIGH (based on codebase reading, live session JSONL analysis, Claude Code docs)

---

## Standard Stack

| Concern | Use | Notes |
|---------|-----|-------|
| Session parsing | `assay_core::context::parse_session` | Phase 20 parser, returns `Vec<ParsedEntry>` with `SessionEntry` enum |
| Session discovery | `assay_core::context::discover_sessions`, `find_session_dir` | Finds `.jsonl` files by project slug |
| Types | `assay_types::context::*` | `SessionEntry`, `ContentBlock`, `EntryMetadata` already exist |
| New checkpoint types | `assay_types::checkpoint` | New module: `TeamCheckpoint`, `AgentState`, `TaskState` |
| Serialization | `serde` + `serde_json` (frontmatter) | Already workspace deps |
| CLI | `clap` derive | Already workspace dep, add `Checkpoint` subcommand |
| Timestamps | `chrono` | Already workspace dep |
| File I/O | `std::fs` | No new deps needed |
| Hook scripts | Bash + `jq` | Consistent with existing plugin hooks |
| Config discovery | `dirs::home_dir()` + glob | `dirs` already workspace dep |

**No new crate dependencies required.**

## Architecture Patterns

### 1. Team State Extractor (assay-core)

New module: `crates/assay-core/src/checkpoint/` with:
- `mod.rs` -- public API (`extract_team_state`, `save_checkpoint`)
- `extractor.rs` -- JSONL scanning for agent/task state
- `config.rs` -- `~/.claude/teams/*/config.json` discovery and merge

The extractor scans session JSONL entries for:
1. **Agent discovery:** `isSidechain: true` + `agentId` field on entries identifies subagents. The `agentId` field (e.g., `"a576d70becbef1223"`) uniquely identifies each subagent. Subagent JSONL files live in `{session_dir}/{session_id}/subagents/agent-{agentId}.jsonl`.
2. **Task operations:** `ContentBlock::ToolUse` with `name: "TaskCreate"` or `name: "TaskUpdate"` in progress entries. TaskCreate input has `{subject, description, activeForm}`. TaskUpdate input has `{taskId, status}` where status is `"in_progress"` | `"completed"` | `"cancelled"`.
3. **Compact boundaries:** `SessionEntry::System` with `subtype: "compact_boundary"` in the `data` field signals context compaction. The `compactMetadata` includes `trigger` ("auto"/"manual") and `preTokens` count.

**Pattern:** Follow the same structure as `crates/assay-core/src/context/` -- separate parser concern from extraction concern. The extractor builds on `parse_session()` output.

### 2. Checkpoint File Format (Dual-Purpose)

Use YAML frontmatter + markdown body. This serves both machine parsing (frontmatter) and human readability (body).

```markdown
---
version: 1
session_id: "0509db4c-b52e-456b-b6f3-8e5578ee608f"
project: "/Users/wollax/Git/personal/assay"
timestamp: "2026-03-06T21:20:30Z"
trigger: "hook:PostToolUse:TaskUpdate"
agent_count: 3
task_count: 4
---

# Team Checkpoint

**Session:** 0509db4c
**Project:** assay
**Captured:** 2026-03-06T21:20:30Z
**Trigger:** PostToolUse:TaskUpdate

## Agents

| Name | Model | Status | Current Task | Working Dir |
|------|-------|--------|-------------|-------------|
| primary | claude-opus-4-6 | active | Implement auth flow | /Users/wollax/Git/personal/assay |
| a576d70becbef1223 | claude-opus-4-6 | active | Task 1: Add path field | /Users/wollax/Git/personal/assay |
| a01021ae91fd3f7cf | claude-opus-4-6 | done | Verify phase 12 | /Users/wollax/Git/personal/assay |

## Tasks

| ID | Name | Status | Assigned Agent | Last Update |
|----|------|--------|----------------|-------------|
| 1 | Add path field and update dispatch | in_progress | a576d70becbef1223 | 2026-03-04T17:31:58Z |
| 2 | Update tests and snapshots | pending | - | 2026-03-04T17:32:01Z |

## Context Health

- **Context tokens:** 168,576 / 200,000 (84.3%)
- **Last compaction:** 2026-03-06T21:20:30Z (auto, pre: 168,576 tokens)
```

### 3. File Location and Lifecycle

| Path | Purpose |
|------|---------|
| `.assay/checkpoints/latest.md` | Rolling "latest" -- always overwritten |
| `.assay/checkpoints/archive/{timestamp}.md` | Timestamped snapshots (ISO 8601 filename) |

**Git tracking:** Add `.assay/checkpoints/` to `.assay/.gitignore`. Checkpoints are operational artifacts, not source. They contain machine-local paths and ephemeral session IDs. Downstream phases (guard daemon) need them to be fast-writable without git overhead.

**Retention:** Keep last 50 archived checkpoints by default. Configurable via `config.toml` future extension (not this phase). Simple count-based pruning on each archive write -- delete oldest beyond limit.

### 4. Hook Integration Pattern

Hooks are configured in `plugins/claude-code/hooks/hooks.json`. Each hook receives JSON on stdin with common fields (`session_id`, `cwd`, `hook_event_name`) plus event-specific fields.

**PostToolUse hooks** receive: `tool_name`, `tool_input` (the tool's input object), `tool_result`, `tool_use_id`. The `matcher` regex filters on tool name. Use matcher `"Task|TaskCreate|TaskUpdate"` to capture all task-related tool uses.

**PreCompact hooks** receive: `hook_event_name: "PreCompact"`, `trigger` ("auto"/"manual"). PreCompact only supports `type: "command"` hooks, not prompt hooks. This is a fire-and-forget event -- stderr shown to user, no blocking capability.

**Stop hooks** receive: `hook_event_name: "Stop"`, `stop_hook_active` (bool for loop prevention), `reason`. Stop hooks CAN block via `{"decision": "block", "reason": "..."}` but checkpoint hooks should NOT block -- just save state.

**Hook script pattern:** A single `checkpoint-hook.sh` script that:
1. Reads stdin JSON
2. Guards: skip if `assay` binary not found, skip if no `.assay/` directory
3. Calls `assay checkpoint --trigger "$HOOK_EVENT_NAME:$TOOL_NAME" --session "$SESSION_ID"`
4. Exits 0 always (never block agent workflow)

**Sync model:** All checkpoint hooks should be fire-and-forget (exit 0 immediately after spawning `assay checkpoint` in background). Timeout of 5s for PostToolUse, 10s for PreCompact/Stop.

**Debouncing:** Time-based debounce of 5 seconds. The hook script checks a timestamp file (`.assay/checkpoints/.last-checkpoint-ts`). If less than 5 seconds since last checkpoint, skip. This prevents rapid-fire checkpoints during batch TaskCreate/TaskUpdate sequences.

### 5. Config.json Discovery

**Location:** `~/.claude/teams/{team_name}/` directories. Observed structure:
```
~/.claude/teams/
  default/
    inboxes/
      team-lead.json    # Inbox messages between agents
```

**Discovery:** The team config is NOT in a `config.json` file in the observed filesystem. The `~/.claude/teams/` directory contains team inboxes (message passing between agents), not team configuration per se. The actual team membership and agent roles are embedded in the session JSONL entries themselves.

**Revised approach:** Instead of relying on `config.json` (which may not exist in the expected format), extract team state entirely from session JSONL:
- Agent list: scan for unique `agentId` values in entries with `isSidechain: true`
- Agent model: from `message.model` field in assistant entries per agent
- Agent status: inferred from entry recency and task state
- Inbox messages: optionally read from `~/.claude/teams/{team}/inboxes/*.json` for coordination context

**Merge strategy (per CONTEXT.md decision):** If `config.json` exists and has team metadata, use it as the authoritative base for agent names/roles. Enrich with runtime data from session JSONL (current task, status, last activity timestamp).

### 6. CLI Command Pattern

Add `assay checkpoint` as a top-level subcommand (same level as `gate`, `spec`, `context`):

```rust
/// Team state checkpointing
Checkpoint {
    #[command(subcommand)]
    command: CheckpointCommand,
}

enum CheckpointCommand {
    /// Take a team state snapshot now
    Save {
        /// Trigger label (e.g., "manual", "hook:Stop")
        #[arg(long, default_value = "manual")]
        trigger: String,
        /// Session ID to checkpoint (default: most recent)
        #[arg(long)]
        session: Option<String>,
        /// Output as JSON instead of summary
        #[arg(long)]
        json: bool,
    },
    /// Show the latest checkpoint
    Show {
        /// Output as JSON (frontmatter only)
        #[arg(long)]
        json: bool,
    },
    /// List archived checkpoints
    List {
        /// Maximum entries to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}
```

### 7. Solo Agent Behavior

**Include solo agent checkpoints.** Even without a team, a checkpoint captures useful recovery context:
- Session ID, project, timestamp
- Context health (token utilization)
- Last compaction boundary
- Active task (if using Task tools in solo mode)

The checkpoint format handles this gracefully -- the agents table just has a single "primary" entry with `isSidechain: false`.

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| JSONL parsing | `assay_core::context::parse_session()` -- already battle-tested |
| Session discovery | `assay_core::context::find_session_dir()` / `discover_sessions()` |
| Home directory | `dirs::home_dir()` -- already used in discovery.rs |
| Project slug | `assay_core::context::discovery::path_to_project_slug()` |
| ISO 8601 timestamps | `chrono::Utc::now().to_rfc3339()` |
| YAML frontmatter parsing | Simple `---` delimiter split + `serde_json` (frontmatter is JSON-compatible YAML subset) |
| File atomicity | Write to temp file + rename (same pattern as history save) |

## Common Pitfalls

### 1. Subagent JSONL Lives in Subdirectories, Not the Main File
Agent session data is in `{session_dir}/{session_id}/subagents/agent-{agentId}.jsonl`, NOT in the main session `.jsonl` file. The main file contains `type: "progress"` entries with `data.type: "agent_progress"` that embed subagent messages, but the full subagent conversation is in the separate files. **The extractor must read both the main session file (for progress entries with TaskCreate/TaskUpdate) AND the subagent files (for agent detail).**

### 2. Task Tool Uses Are Embedded in Progress Entries
TaskCreate/TaskUpdate tool uses appear as `ContentBlock::ToolUse` inside `data.message.message.content` of `type: "progress"` entries with `data.type: "agent_progress"`. They are NOT direct assistant entries. The current `SessionEntry::Progress` type stores `data` as `Option<serde_json::Value>`, so extraction requires JSON traversal of the `data` field.

### 3. Hook Timeout Kills the Process
If a checkpoint hook exceeds its timeout, Claude Code kills it. The hook script must spawn `assay checkpoint` in background and exit immediately, or the checkpoint operation must be fast enough to complete within the timeout window. For PostToolUse with 5s timeout, background spawn is safer.

### 4. `stop_hook_active` Must Be Checked
The Stop hook receives `stop_hook_active: true` when a stop hook is currently running (to prevent infinite loops). The existing `stop-gate-check.sh` already handles this. The checkpoint hook must also check this flag and skip if true.

### 5. PreCompact Cannot Block
Unlike Stop hooks which can return `{"decision": "block"}`, PreCompact hooks are informational only. Output goes to stderr visible to the user. The checkpoint hook should just save and exit 0.

### 6. Config.json May Not Exist
The `~/.claude/teams/` directory structure is not guaranteed. The extractor must handle:
- No `~/.claude/teams/` directory
- Empty team directories
- Inbox files with unexpected format

Always fall back to session-only extraction.

### 7. Checkpoint Write Must Be Atomic
Use write-to-temp-then-rename to avoid half-written checkpoints if the process is killed mid-write. The `latest.md` file is read by other processes (future guard daemon), so it must always be in a valid state.

### 8. Session ID Directory vs File Ambiguity
Sessions in `~/.claude/projects/{slug}/` can be either a `.jsonl` file directly OR a directory with subagents. When a session has subagents, the structure is:
```
{slug}/{session_id}.jsonl          # Main session file
{slug}/{session_id}/               # Session directory
  subagents/agent-{id}.jsonl       # Subagent files
  tool-results/{id}.txt            # Tool result storage
```
Both the `.jsonl` file and the directory exist for the same session.

## Code Examples

### Extracting TaskCreate from Progress Entries

```rust
/// Extract task operations from a parsed session.
fn extract_task_ops(entries: &[ParsedEntry]) -> Vec<TaskOperation> {
    let mut ops = Vec::new();
    for parsed in entries {
        let SessionEntry::Progress(progress) = &parsed.entry else { continue };
        let Some(data) = &progress.data else { continue };

        // Navigate: data.message.message.content[].{name, input}
        let content = data
            .pointer("/message/message/content")
            .and_then(|c| c.as_array());
        let Some(blocks) = content else { continue };

        for block in blocks {
            let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let input = block.get("input");
            let agent_id = data.get("agentId").and_then(|a| a.as_str());

            match name {
                "TaskCreate" => {
                    if let Some(input) = input {
                        ops.push(TaskOperation::Create {
                            subject: input.get("subject").and_then(|s| s.as_str())
                                .unwrap_or("").to_string(),
                            description: input.get("description").and_then(|s| s.as_str())
                                .unwrap_or("").to_string(),
                            agent_id: agent_id.map(String::from),
                            timestamp: parsed_timestamp(&progress.meta),
                        });
                    }
                }
                "TaskUpdate" => {
                    if let Some(input) = input {
                        ops.push(TaskOperation::Update {
                            task_id: input.get("taskId").and_then(|s| s.as_str())
                                .unwrap_or("").to_string(),
                            status: input.get("status").and_then(|s| s.as_str())
                                .unwrap_or("").to_string(),
                            agent_id: agent_id.map(String::from),
                            timestamp: parsed_timestamp(&progress.meta),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    ops
}
```

### Checkpoint Hook Script

```bash
#!/usr/bin/env bash
# Checkpoint hook: save team state on task operations, compaction, or stop.
# Fires on PostToolUse[Task|TaskCreate|TaskUpdate], PreCompact, Stop.
# NEVER blocks agent workflow.

INPUT=$(cat)

# Guard: assay binary must exist
command -v assay &>/dev/null || exit 0

# Guard: .assay/ directory must exist
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')
[ -z "$CWD" ] || [ ! -d "$CWD/.assay" ] && exit 0

# Guard: stop_hook_active loop prevention
STOP_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
[ "$STOP_ACTIVE" = "true" ] && exit 0

# Debounce: skip if last checkpoint was < 5s ago
LAST_TS_FILE="$CWD/.assay/checkpoints/.last-checkpoint-ts"
if [ -f "$LAST_TS_FILE" ]; then
  LAST_TS=$(cat "$LAST_TS_FILE")
  NOW=$(date +%s)
  DIFF=$((NOW - LAST_TS))
  [ "$DIFF" -lt 5 ] && exit 0
fi

# Extract trigger info
EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // "unknown"')
TOOL=$(echo "$INPUT" | jq -r '.tool_name // empty')
SESSION=$(echo "$INPUT" | jq -r '.session_id // empty')
TRIGGER="${EVENT}${TOOL:+:$TOOL}"

# Run checkpoint in background (fire-and-forget)
cd "$CWD" || exit 0
assay checkpoint save --trigger "$TRIGGER" ${SESSION:+--session "$SESSION"} &>/dev/null &

exit 0
```

### hooks.json Integration

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [{ "type": "command", "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/post-tool-use.sh", "timeout": 5 }]
      },
      {
        "matcher": "Task|TaskCreate|TaskUpdate",
        "hooks": [{ "type": "command", "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/checkpoint-hook.sh", "timeout": 5 }]
      }
    ],
    "PreCompact": [
      {
        "hooks": [{ "type": "command", "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/checkpoint-hook.sh", "timeout": 10 }]
      }
    ],
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/stop-gate-check.sh", "timeout": 120 },
          { "type": "command", "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/checkpoint-hook.sh", "timeout": 10 }
        ]
      }
    ]
  }
}
```

## Decisions for Planner

### 1. Checkpoint types go in `assay-types/src/checkpoint.rs` (new module)
Types: `TeamCheckpoint`, `AgentState`, `TaskState`, `CheckpointTrigger`. All derive `Serialize, Deserialize, JsonSchema`. Register with `inventory::submit!`.

### 2. Extractor logic goes in `assay-core/src/checkpoint/` (new module)
Separate from `context/` because checkpoint extraction has different concerns (team state assembly vs token diagnostics). Reuses `context::parse_session` as input.

### 3. Use YAML frontmatter + markdown body format
Frontmatter is machine-parseable (downstream guard daemon needs it). Body is human-readable (developers reviewing checkpoints). Use `serde_json` for frontmatter (JSON is valid YAML).

### 4. Solo agents get checkpoints too
A solo session checkpoint captures context health and session metadata. The agent table has one entry. This is valuable for PreCompact checkpoints especially (capturing state before compaction erases context).

### 5. Single hook script for all three events
One `checkpoint-hook.sh` handles PostToolUse, PreCompact, and Stop. Event type is passed via `$HOOK_EVENT_NAME` in stdin JSON. Reduces maintenance surface.

### 6. Background execution for safety
The hook script spawns `assay checkpoint save` in background and exits 0 immediately. This guarantees the hook never exceeds timeout and never blocks the agent.

### 7. `.assay/checkpoints/` is gitignored
Add to `.assay/.gitignore`. Checkpoints contain machine-local paths and ephemeral session data.

## Test Strategy

| What | How | Confidence |
|------|-----|------------|
| Type serialization roundtrip | `serde_json` roundtrip tests in `assay-types` | HIGH |
| Task extraction from progress entries | Unit test with crafted JSONL containing TaskCreate/TaskUpdate in progress data | HIGH |
| Agent discovery from sidechain entries | Unit test with entries having `isSidechain: true` and `agentId` | HIGH |
| Checkpoint file write/read | Integration test: `save_checkpoint` then `load_checkpoint`, verify frontmatter + body | HIGH |
| Latest file atomic overwrite | Test that concurrent reads never see partial content (write to temp + rename) | MEDIUM |
| CLI `checkpoint save` | Integration test with tempdir `.assay/` and sample session | HIGH |
| Hook script guards | Unit test each guard condition (no binary, no .assay/, debounce) | MEDIUM |
| Compact boundary extraction | Unit test with system entry containing `compact_boundary` subtype | HIGH |
| Solo agent checkpoint | Test that a session with no subagents still produces valid checkpoint | HIGH |
| Config.json merge | Test with mock `~/.claude/teams/` directory; test graceful fallback when missing | MEDIUM |

---

*Phase: 21-team-state-checkpointing*
*Research completed: 2026-03-06*
