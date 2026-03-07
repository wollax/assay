---
phase: 24
plan: "24-01"
subsystem: mcp
tags: [history, persistence, gate-run, parity]
dependency-graph:
  requires: [14-01, 15-01, 17-01, 19-02]
  provides: [command-only-mcp-history-persistence]
  affects: [25-schema-roundtrip]
tech-stack:
  added: []
  patterns: [non-fatal-save, tracing-warn-on-failure]
key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs
    - crates/assay-mcp/tests/mcp_handlers.rs
decisions:
  - "format_gate_response takes &GateRunSummary (reference), so no clone needed before history save"
  - "Save failures are non-fatal — logged via tracing::warn, matching timeout handler pattern"
  - "summary.clone() used only for GateRunRecord construction (summary is borrowed by format_gate_response via reference)"
metrics:
  duration: ~5m
  completed: 2026-03-07
---

# Phase 24 Plan 01: Command-Only MCP History Persistence Summary

**One-liner:** Added history persistence for command-only MCP gate_run, closing the CLI/MCP parity gap where command-only specs never saved run records.

## What Changed

The MCP `gate_run` handler had an asymmetry: specs with agent criteria saved history via `gate_finalize`, but command-only specs (no `AgentReport` criteria) bypassed that flow entirely. Their runs were never recorded.

Added an `else` branch after the `if let Some(info) = agent_info { ... }` block that:
1. Constructs a `GateRunRecord` with timestamp, run_id, working_dir, and the evaluation summary
2. Calls `assay_core::history::save()` to persist the record
3. Logs failures via `tracing::warn!` (non-fatal, matching the pattern used in the timeout handler)

## Tasks Completed

| # | Task | Commit |
|---|------|--------|
| 1 | Add history save for command-only gate_run | f054e8c |
| 2 | Integration test for command-only history persistence | 8d4ac8a |
| - | rustfmt fix | 7cec496 |

## Decisions Made

1. **No clone needed for format_gate_response** — it takes `&GateRunSummary`, so `summary` remains available for `GateRunRecord` construction. The `summary.clone()` in the record is needed because the record takes ownership.
2. **Non-fatal save pattern** — matches the existing timeout handler's `tracing::warn!` approach, not the CLI's `eprintln!` approach (MCP server should use structured logging).

## Deviations

None — plan executed exactly as written.

## Verification

- `cargo test -p assay-mcp --test mcp_handlers gate_run_command_only_persists_history` passes
- All 8 MCP handler integration tests pass
- `just ready` passes (fmt-check, lint, test, deny)
