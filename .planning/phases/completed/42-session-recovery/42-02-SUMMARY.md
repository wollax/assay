---
phase: 42
plan: 02
subsystem: session-management
tags: [recovery, mcp, startup, sessions]
dependency_graph:
  requires: ["42-01"]
  provides: ["recover_stale_sessions", "RecoverySummary", "MCP startup recovery"]
  affects: ["43"]
tech_stack:
  added: ["hostname 0.4"]
  patterns: ["startup recovery scan", "graceful degradation on corrupt data"]
key_files:
  created: []
  modified:
    - crates/assay-core/src/work_session.rs
    - crates/assay-mcp/src/server.rs
decisions:
  - "Recovery runs in serve() before transport binding, not in AssayServer::new()"
  - "Staleness measured from AgentRunning transition timestamp, not created_at"
  - "Recovery scan capped at 100 sessions (oldest first via ULID sort)"
  - "load_recovery_threshold reads [sessions].stale_threshold from config.toml with 3600 default"
  - "Recovery never fatal — all errors logged and skipped"
metrics:
  completed: 2026-03-15
---

# Phase 42 Plan 02: Startup Recovery Scan for Stale Sessions Summary

Recovery scan detects orphaned AgentRunning sessions on MCP server startup and marks them Abandoned with host/PID/timing notes. Wired into serve() before transport binding.

## What Was Built

### Task 1: Recovery scan in assay-core

- `RecoverySummary` struct tracking recovered/skipped/errors counts
- `format_duration` helper for human-readable duration strings (e.g., "3h 12m")
- `build_recovery_note` formatting recovery notes with stale duration, threshold, hostname, PID
- `recover_stale_sessions` scanning sessions dir, filtering AgentRunning sessions past threshold, transitioning to Abandoned with recovery notes
- 10 tests covering: empty dir, no dir, stale recovery, non-AgentRunning skip, fresh session skip, corrupt file handling, idempotency, missing transition record, note formatting

### Task 2: MCP server wiring

- `load_recovery_threshold` helper reading `[sessions].stale_threshold` from config.toml with 3600 default
- Recovery call in `serve()` before `AssayServer::new().serve(stdio())` — only runs when sessions dir exists
- Recovery summary logged only when recovered > 0 or errors > 0

## Commits

- `a087c95`: feat(42-02): implement recovery scan for stale sessions
- `6f083a7`: feat(42-02): wire recovery scan into MCP server startup

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- [x] `just ready` passes (fmt, clippy, test, deny)
- [x] All 34 work_session tests pass including 10 new recovery tests
- [x] Recovery correctly marks stale AgentRunning sessions as Abandoned
- [x] Recovery note includes hostname, PID, stale duration, threshold
- [x] Corrupt files logged and skipped
- [x] Non-AgentRunning sessions untouched
- [x] Fresh sessions untouched
- [x] Idempotent (second run recovers nothing)
- [x] Staleness measured from AgentRunning transition timestamp
- [x] Missing transition records warned and skipped
- [x] Scan capped at 100 sessions
- [x] serve() calls recovery before binding transport
- [x] Threshold read from config with 3600 default
