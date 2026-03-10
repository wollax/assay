---
phase: 33-mcp-validation
plan: 02
subsystem: mcp
tags: [mcp, gate-run, stdout-fallback, clone-elimination]
dependency-graph:
  requires: []
  provides: [stdout-fallback-failure-reason, clone-free-gate-run]
  affects: []
tech-stack:
  added: []
  patterns: [stderr-first-stdout-fallback, partial-move-from-struct]
key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs
decisions:
  - id: MCP-04
    summary: "Stderr-first, stdout-fallback chain for failure reason extraction"
  - id: MCP-05
    summary: "Partial struct moves to eliminate GateRunSummary and Vec<CriterionResult> clones"
metrics:
  duration: ~7m
  completed: 2026-03-10
---

# Phase 33 Plan 02: Stdout Fallback + Clone Elimination Summary

Stderr-first stdout-fallback for MCP failure reasons, plus partial-move clone elimination in gate_run handler.

## What Was Done

### Task 1: Stdout Fallback for Failure Reason Extraction (MCP-04)

Changed `format_gate_response` to chain `first_nonempty_line` calls: stderr first, then stdout, then "unknown". This ensures agents get meaningful failure reasons even when commands write diagnostics to stdout instead of stderr.

**Commits:** `d6ac4ed`

Three new tests added:
- `test_failure_reason_prefers_stderr` — both populated, reason comes from stderr
- `test_failure_reason_falls_back_to_stdout` — empty stderr, reason comes from stdout
- `test_failure_reason_both_empty_shows_unknown` — both empty, reason is "unknown"

### Task 2: Clone Elimination in gate_run Handler (MCP-05)

Eliminated two unnecessary clones:
1. `summary.results.clone()` → moved `summary.results` directly into `create_session()`
2. `summary.clone()` → moved `summary` directly into `save_run()`, with `spec_name` extracted beforehand for the tracing warn

**Commits:** `cad4afe`

## Deviations from Plan

None — plan executed exactly as written.

## Decisions Made

| ID | Decision | Rationale |
|----|----------|-----------|
| MCP-04 | Stderr-first, stdout-fallback chain | Many CLI tools write errors to stdout; agents need actionable failure reasons |
| MCP-05 | Partial struct moves over cloning | Rust allows moving fields out of a struct when the struct is not used afterward |

## Verification

- `just ready` passes (fmt-check, lint, test, deny)
- All existing tests pass unchanged
- Three new tests validate the stdout fallback behavior

## Next Phase Readiness

No blockers. Ready for remaining Phase 33 plans.
