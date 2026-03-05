# 17-02 Summary: Gate History Tool & Response Documentation

---
phase: 17
plan: 02
subsystem: assay-mcp
tags: [mcp, history, documentation]
depends_on: ["01"]
tech_stack: [rust, rmcp, serde, schemars]
key_files:
  - crates/assay-mcp/src/server.rs
decisions:
  - "gate_history loads config for consistency but prefixes with _ (not directly needed beyond validation)"
  - "List mode returns most-recent-first by reversing the oldest-first list from history::list()"
  - "Unreadable history entries are skipped with tracing::warn, not fatal errors"
  - "Default limit is 10 runs in list mode"
  - "Detail mode returns the raw GateRunRecord JSON (full fidelity, no mapping)"
metrics:
  tasks_completed: 2
  tasks_total: 2
  duration_seconds: 238
  files_modified: 1
  insertions: 171
  deletions: 12
---

Added the `gate_history` MCP tool for querying past gate run results and documented all response struct fields across the MCP server.

## Task Results

### Task 1: Implement gate_history MCP tool
- **Commit:** `3d12ab8`
- **Result:** Pass
- Added `GateHistoryParams` with `name`, `run_id` (optional), `limit` (optional, default 10)
- Added `GateHistoryListResponse` and `GateHistoryEntry` response structs with full doc comments
- List mode: calls `history::list()` + `history::load()` for each, returns `{spec_name, total_runs, runs}`
- Detail mode: calls `history::load()` directly, returns full `GateRunRecord` JSON
- Unreadable entries silently skipped with `tracing::warn`
- Updated module doc (five→six tools) and server instructions to mention `gate_history`

### Task 2: Add doc comments to all response structs and review tool descriptions
- **Commit:** `9d62bbd`
- **Result:** Pass
- Added `///` doc comments to every field on: `SpecListEntry`, `SpecListResponse`, `SpecListError`, `GateRunResponse`, `GateReportResponse`, `CriterionSummary`
- `GateHistoryListResponse` and `GateHistoryEntry` already documented in Task 1
- Updated `spec_list` description: mentions envelope format `{specs, errors?}` with field details
- Updated `gate_run` description: mentions timeout parameter, enforcement-level counts, blocked flag
- `gate_history` description already accurate from Task 1
- `just ready` passes (fmt-check + lint + test + deny)

## Deviations

None.

## Decisions Made

- `gate_history` validates project config (loads config) for consistency with other tools, even though it only needs `.assay/results/` path. Prefixed `_config` to suppress unused warning.
- List mode reverses `history::list()` output (oldest-first → most-recent-first) for agent ergonomics.
- Detail mode passes through the full `GateRunRecord` without mapping, maximizing fidelity.
