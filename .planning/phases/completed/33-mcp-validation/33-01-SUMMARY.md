---
phase: 33-mcp-validation
plan: 01
subsystem: mcp-server
tags: [mcp, serde, validation, regression-tests]
dependency-graph:
  requires: [31-error-messages]
  provides: [mcp-param-validation-regression-tests, mcp-spec-not-found-regression-test]
  affects: [33-02]
tech-stack:
  added: []
  patterns: [serde-deserialization-testing]
key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs
    - crates/assay-core/src/spec/mod.rs
decisions:
  - Used `.err().unwrap()` instead of `.unwrap_err()` to avoid needing `Debug` derives on param structs
  - MCP-03 Display-chain tests placed in assay-core (where the tested function lives) rather than assay-mcp
metrics:
  duration: ~9 minutes
  completed: 2026-03-10
---

# Phase 33 Plan 01: Serde Validation Regression Tests Summary

Regression tests proving rmcp serde deserialization and spec-not-found diagnostics produce specific, parameter-naming error messages without custom validation code.

## What Was Done

### Task 1: MCP-01 and MCP-02 Serde Deserialization Tests (0170ddf)

Added 7 unit tests to `crates/assay-mcp/src/server.rs` verifying serde error messages:

**MCP-01 (missing required parameters) — 4 tests:**
- `test_gate_run_params_missing_name` — error contains "missing field" and "name"
- `test_gate_report_params_missing_fields` — error contains "missing field"
- `test_spec_get_params_missing_name` — error contains "missing field" and "name"
- `test_gate_finalize_params_missing_session_id` — error contains "missing field" and "session_id"

**MCP-02 (invalid parameter types) — 3 tests:**
- `test_gate_run_params_invalid_timeout_type` — error contains "invalid type" for string-as-u64
- `test_gate_report_params_invalid_passed_type` — error contains "invalid type" and "bool"
- `test_gate_run_params_invalid_include_evidence_type` — error contains "invalid type" and "bool"

### Task 2: MCP-03 Spec-Not-Found Display Chain Test (d6ac4ed)

Added 2 tests to `crates/assay-core/src/spec/mod.rs` verifying the full chain from `load_spec_entry_with_diagnostics` through `err.to_string()`:

- `spec_not_found_display_includes_available_specs` — error Display contains "not found" and names of available specs
- `spec_not_found_display_empty_dir` — error Display contains "No specs found"

## Decisions Made

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Used `.err().unwrap()` pattern | Avoids adding `Debug` derives to param structs just for tests |
| 2 | MCP-03 tests in assay-core | Tests `load_spec_entry_with_diagnostics` + Display impl, which live in assay-core |
| 3 | Used `"self"` not `"agent"` for evaluator_role in test | EvaluatorRole uses kebab-case serde rename; valid variants are self/independent/human |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed invalid evaluator_role in test JSON**
- **Found during:** Task 1
- **Issue:** Plan specified `"evaluator_role": "agent"` but EvaluatorRole enum only accepts `"self"`, `"independent"`, `"human"` (kebab-case). Serde failed on the role before reaching the `passed` field.
- **Fix:** Changed to `"evaluator_role": "self"` so serde reaches the `passed` field and reports the type mismatch.
- **Files modified:** `crates/assay-mcp/src/server.rs`
- **Commit:** 0170ddf

**2. [Rule 3 - Blocking] Used `.err().unwrap()` instead of `.unwrap_err()`**
- **Found during:** Task 1
- **Issue:** `unwrap_err()` requires `T: Debug`, and param structs don't derive `Debug`. Adding `Debug` would be scope creep.
- **Fix:** Used `.err().unwrap()` which only requires the Result to be an Option, avoiding the Debug bound.
- **Files modified:** `crates/assay-mcp/src/server.rs`
- **Commit:** 0170ddf

## Verification

`just ready` passes: fmt-check, lint, test (all 414 core tests + 60 mcp tests), deny.

## Next Phase Readiness

No blockers. Plan 33-02 can proceed independently.
