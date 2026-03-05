---
phase: 17
plan: 01
subsystem: mcp
tags: [mcp, timeout, validation, error-handling, enforcement]
dependency-graph:
  requires: [16]
  provides: [hardened-gate-run, spec-list-errors, enforcement-response-fields]
  affects: [17-02]
tech-stack:
  added: []
  patterns: [tokio-timeout-wrapping, error-envelope-pattern]
key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs
decisions:
  - id: 17-01-01
    summary: "gate_run timeout defaults to 300s, returns CallToolResult error (not McpError) on expiry"
  - id: 17-01-02
    summary: "working_dir validation via is_dir() check before spawn_blocking — early return with domain error"
  - id: 17-01-03
    summary: "spec_list uses SpecListResponse envelope with skip_serializing_if on errors vec"
  - id: 17-01-04
    summary: "GateRunResponse gains required_passed, advisory_passed, blocked fields computed from EnforcementSummary"
metrics:
  duration: "~4 minutes"
  completed: 2026-03-05
---

# Phase 17 Plan 01: MCP Tool Hardening Summary

**One-liner:** Hardened gate_run with tokio timeout wrapping and path validation, spec_list with error envelope, and enforcement count fields on GateRunResponse.

## What Was Done

### Task 1: Timeout and Path Validation for gate_run

- Added `timeout: Option<u64>` field to `GateRunParams` with schemars description
- Added `std::time::Duration` import
- Inserted `working_dir.is_dir()` check before `spawn_blocking` — returns `CallToolResult::error` (domain error, not McpError)
- Wrapped `spawn_blocking` with `tokio::time::timeout` using the timeout param (default 300s)
- Timeout expiry returns a descriptive `CallToolResult::error` with elapsed seconds
- Added 2 deserialization tests for GateRunParams with/without timeout field

### Task 2: Spec List Errors and Enforcement Response Fields

- Added `SpecListResponse` envelope struct with `specs` and `errors` fields
- Added `SpecListError` struct with `message` field
- Updated `spec_list` to collect `scan_result.errors` into `Vec<SpecListError>` and serialize the envelope
- `errors` field omitted from JSON when empty via `skip_serializing_if = "Vec::is_empty"`
- Added `required_passed`, `advisory_passed`, and `blocked` fields to `GateRunResponse`
- `blocked` computed as `required_failed > 0`
- Updated `format_gate_response` to populate all enforcement fields from `EnforcementSummary`
- Updated `sample_summary()` test helper to use accurate enforcement values
- Updated 3 existing test constructions to include new fields
- Added assertions for new fields in existing tests
- Added 3 new tests: SpecListResponse with/without errors, enforcement counts verification

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated sample_summary enforcement values**

- **Found during:** Task 2
- **Issue:** `sample_summary()` used `EnforcementSummary::default()` (all zeros) which was inaccurate for its 1-passed/1-failed/1-skipped data
- **Fix:** Set accurate enforcement values (required_passed: 1, required_failed: 1)
- **Files modified:** crates/assay-mcp/src/server.rs

## Verification

- 32 tests passing (up from 29 at start)
- Zero clippy warnings
- Formatting clean

## Next Phase Readiness

Plan 17-02 can proceed. All prerequisite hardening from 17-01 is in place:
- Timeout infrastructure ready for gate_run
- Path validation protects against invalid working directories
- Error envelope pattern established for spec_list
- Enforcement fields available in GateRunResponse for agent consumption
