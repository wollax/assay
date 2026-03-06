---
phase: 21
plan: 01
subsystem: checkpoint
tags: [checkpoint, team-state, extraction, persistence, serde, jsonl]
requires: [20]
provides: [checkpoint-types, checkpoint-extraction, checkpoint-persistence]
affects: [21-02, 21-03]
tech-stack:
  added: []
  patterns: [json-frontmatter, atomic-write, entry-scanning]
key-files:
  created:
    - crates/assay-types/src/checkpoint.rs
    - crates/assay-core/src/checkpoint/mod.rs
    - crates/assay-core/src/checkpoint/extractor.rs
    - crates/assay-core/src/checkpoint/config.rs
    - crates/assay-core/src/checkpoint/persistence.rs
  modified:
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/lib.rs
    - crates/assay-core/src/error.rs
decisions:
  - ParsedEntry imported via pub re-export (crate::context::ParsedEntry), not private parser module
  - merge_team_config is a no-op until team config.json format stabilizes; session-extracted state is authoritative
  - JSON frontmatter (not YAML) between --- delimiters for machine-parseable checkpoints
  - Archive filenames use ISO 8601 with colons replaced by dashes for filesystem compatibility
  - Context health uses fixed 200K context window (same as tokens module)
metrics:
  duration: 343s
  completed: 2026-03-06
---

# Phase 21 Plan 01: Checkpoint Types and Core Logic Summary

Checkpoint types in assay-types and extraction + persistence logic in assay-core for team state snapshots from session JSONL files.

## What Was Built

### Types (assay-types)
- `TeamCheckpoint` — top-level checkpoint with version, session_id, project, timestamp, trigger, agents, tasks, context_health
- `AgentState` — agent_id, model, status, current_task, working_dir, is_sidechain, last_activity
- `AgentStatus` enum — Active, Idle, Done, Unknown
- `TaskState` — task_id, subject, description, status, assigned_agent, last_update
- `TaskStatus` enum — Pending, InProgress, Completed, Cancelled
- `ContextHealthSnapshot` — context_tokens, context_window, utilization_pct, last_compaction, compaction_trigger
- All registered with `inventory::submit!` for schema generation

### Extraction (assay-core)
- `extract_team_state()` — full pipeline: find session dir, resolve session, parse JSONL, extract agents/tasks/compaction, build context health, merge team config
- `extract_agents()` — primary agent always present; subagents discovered from `isSidechain + agentId` in progress entries
- `extract_tasks()` — TaskCreate/TaskUpdate tool uses extracted from `data.message.message.content` in progress entries
- `extract_compaction()` — compact_boundary detection from system entries

### Team Config (assay-core)
- `discover_team_config()` — reads `~/.claude/teams/*/config.json` and `inboxes/*.json`, returns None gracefully when absent
- `merge_team_config()` — enrichment hook (currently no-op; session data is authoritative until config.json format stabilizes)

### Persistence (assay-core)
- `save_checkpoint()` — atomic writes (tempfile-then-rename) to `latest.md` + `archive/{timestamp}.md`, prunes archive to 50 entries, updates `.last-checkpoint-ts`
- `load_latest_checkpoint()` — parses JSON frontmatter from `latest.md`
- `list_checkpoints()` — lists archive entries newest-first with limit
- `render_checkpoint()` — JSON frontmatter + human-readable markdown body with agents table, tasks table, context health section
- `CheckpointEntry` — persistence-layer summary struct for archive listing

### Error Variants
- `CheckpointWrite { path, message }` — file write failures
- `CheckpointRead { path, message }` — file read/parse failures

## Tests

21 tests across 3 test modules:
- **extractor**: primary-only agents, subagent discovery, task extraction (single + multiple creates, updates), compaction detection (present + absent), solo agent checkpoint
- **config**: discover returns None when no teams dir, merge no-op when None, merge preserves session data when config present
- **persistence**: frontmatter delimiters, frontmatter valid JSON, markdown body content, save/load roundtrip, timestamp file creation, archive pruning (55 -> 50), list newest-first, list respects limit, list empty archive, load missing file error, frontmatter parse rejection

## Decisions Made

1. **ParsedEntry import path**: Used `crate::context::ParsedEntry` (pub re-export) since `parser` module is private
2. **merge_team_config is a deliberate no-op**: Team `config.json` format is not well-defined; session JSONL is the authoritative source for all agent/task state
3. **JSON frontmatter**: Frontmatter is serialized as JSON (valid YAML subset), making parsing unambiguous
4. **Archive filename format**: ISO 8601 with colons replaced by dashes (e.g., `2026-03-06T10-00-00Z.md`) for filesystem compatibility
5. **Context window constant**: Uses 200K (same as `context::tokens` module) rather than model-specific lookup

## Deviations from Plan

None — plan executed exactly as written.

## Next Phase Readiness

Plan 02 (CLI commands) can proceed immediately — all public APIs are in place:
- `extract_team_state(project_dir, session_id, trigger)`
- `save_checkpoint(assay_dir, checkpoint)`
- `load_latest_checkpoint(assay_dir)`
- `list_checkpoints(assay_dir, limit)`
