---
phase: 34
plan: 1
subsystem: mcp
tags: [truncation, mcp, gate-response, serde]
dependency-graph:
  requires: [29, 33]
  provides: [mcp-truncation-visibility]
  affects: []
tech-stack:
  added: []
  patterns: [skip_serializing_if-optional-metadata]
key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs
    - crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap
decisions:
  - truncated field is Option<bool> (Some for evaluated, None for skipped) — mirrors evidence-flag independence
  - original_bytes is Option<u64> with skip_serializing_if so absent when None (not truncated or skipped)
metrics:
  duration: 4m
  completed: 2026-03-10
---

# Phase 34 Plan 1: MCP Truncation Visibility Summary

Expose truncation metadata (truncated, original_bytes) in MCP gate responses so agents can programmatically detect truncated output without parsing in-band text markers.

## What Changed

### CriterionSummary struct (server.rs)
Added two fields after `stderr`:
- `truncated: Option<bool>` — `Some(true/false)` for evaluated criteria, `None` for skipped
- `original_bytes: Option<u64>` — original byte count before truncation, `None` when not truncated or skipped

Both use `#[serde(skip_serializing_if = "Option::is_none")]` for clean JSON output.

### format_gate_response mapping
- **Skipped arm:** `truncated: None, original_bytes: None`
- **Passed arm:** `truncated: Some(gate_result.truncated), original_bytes: gate_result.original_bytes`
- **Failed arm:** same as passed

### Tests
- `test_criterion_summary_truncation_fields_all_states` — verifies struct values and JSON serialization for passed+truncated, failed+not-truncated, and skipped criteria
- `test_truncation_fields_independent_of_include_evidence` — confirms truncation metadata present in both `include_evidence=true` and `include_evidence=false` modes
- Updated insta snapshot for gate_run_command_spec to include `truncated: false`

## Decisions Made

1. `truncated` is `Option<bool>` rather than plain `bool` so skipped criteria omit it entirely from JSON (consistent with other skipped-criterion fields like `kind_label`, `exit_code`, `duration_ms`)
2. `original_bytes` uses `Option<u64>` matching the GateResult type directly — absent when output was not truncated

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated insta snapshot for gate_run_command_spec**
- **Found during:** Task 2 (test verification)
- **Issue:** Adding `truncated: Some(false)` to passed criteria caused the existing snapshot to fail since JSON now includes `"truncated": false`
- **Fix:** Updated snapshot file to include the new field
- **Files modified:** `crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap`

## Verification

`just ready` passes: fmt-check, lint (clippy -D warnings), all tests, cargo-deny.

## Next Phase Readiness

No blockers. This was the final gap closure phase for MCP truncation visibility.
