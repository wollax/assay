---
phase: 41
plan: 01
title: "Session MCP tools: create, get, update, list"
subsystem: assay-mcp
tags: [mcp, session, persistence, work-session]
requires: [40-02]
provides: [session-mcp-tools]
affects: [assay-mcp, assay-core]
tech-stack:
  added: []
  patterns: [mcp-tool-handler, param-struct, response-struct]
key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs
    - crates/assay-mcp/src/lib.rs
decisions: []
metrics:
  duration: ~10m
  completed: 2026-03-15
---

# Phase 41 Plan 01: Session MCP Tools Summary

## What was done

Added four session MCP tools to the AssayServer, wiring them to the Phase 40 persistence layer:

1. **`session_create`** - Creates a new work session for a spec. Validates spec exists (consistent with `gate_run`), persists to `.assay/sessions/`, returns session ID and initial state.

2. **`session_get`** - Retrieves full session data by ID using `#[serde(flatten)]` for the WorkSession type. Provides O(1) lookup vs loading all sessions.

3. **`session_update`** - Transitions session phase and appends gate_run_ids with deduplication. Invalid transitions are rejected via the `can_transition_to` state machine.

4. **`session_list`** - Enumerates sessions with optional `spec_name` (exact match), `status` (phase), and `limit` (default 20, max 100) filters. Returns summary entries ordered chronologically (ULID sort). Reports `total` (pre-filter count) and warns on unreadable sessions.

## Structural additions

- 4 param structs: `SessionCreateParams`, `SessionGetParams`, `SessionUpdateParams`, `SessionListParams`
- 5 response structs: `SessionCreateResponse`, `SessionGetResponse`, `SessionUpdateResponse`, `SessionListEntry`, `SessionListResponse`
- All responses include `warnings: Vec<String>` with `skip_serializing_if` (Phase 35 cross-cutting)
- `get_info()` instructions updated to mention session tools
- Module doc comments updated in both `server.rs` (13 -> 17 tools) and `lib.rs` (8 -> 17 tools)
- Param structs exported under `#[cfg(any(test, feature = "testing"))]` in `lib.rs`

## Test coverage

11 new tests added (all `#[tokio::test] #[serial]`):

| Test | Validates |
|------|-----------|
| `session_create_happy_path` | Creates session, returns ULID ID, phase="created" |
| `session_create_invalid_spec` | Domain error for nonexistent spec |
| `session_get_happy_path` | Full session data returned by ID |
| `session_get_not_found` | Error for nonexistent session ID |
| `session_update_happy_path` | Phase transition created->agent_running |
| `session_update_invalid_transition` | Rejects created->completed skip |
| `session_update_with_gate_run_ids` | IDs appended and deduplicated |
| `session_list_empty` | Returns total=0, sessions=[] |
| `session_list_with_spec_name_filter` | Exact match on spec_name |
| `session_list_with_status_filter` | Phase filter |
| `session_list_respects_limit` | Limit=1 returns 1 of 3 |

## Deviations

None. Plan executed as written.

## Commits

- `b279b1e`: feat(41-01): add session MCP tools (create, get, update, list)
