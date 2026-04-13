---
phase: 68-mcp-surface
plan: 01
subsystem: mcp
tags: [rust, rmcp, mcp, wizard, gate-wizard, criteria-create, spawn-blocking]

# Dependency graph
requires:
  - phase: 67-wizard-core-cli-surface
    provides: apply_gate_wizard and apply_criteria_wizard core functions in assay-core::wizard
  - phase: 64-type-foundation
    provides: GateWizardInput, CriteriaWizardInput, CriterionInput types in assay-types

provides:
  - gate_wizard MCP tool handler with GateWizardResponse struct in server.rs
  - criteria_create MCP tool handler with CriteriaCreateResponse struct in server.rs
  - 6 integration tests covering tool registration, disk writes, and duplicate rejection

affects:
  - 68-02 (criteria_list and criteria_get tools in same phase)
  - any future MCP surface plans that follow the same handler pattern

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reuse assay-types input structs directly as MCP params (Deserialize + JsonSchema) — zero wrapper overhead"
    - "Serialize-only response structs (no Deserialize) — output types are display-only per Phase 67 decision"
    - "spawn_blocking for all file I/O in async MCP handlers"
    - "domain_error() converts AssayError to isError=true CallToolResult"

key-files:
  created: []
  modified:
    - crates/assay-mcp/src/server.rs

key-decisions:
  - "Tests and implementation committed together (single feat commit) because pre-commit hook runs clippy which requires the implementation to exist"

patterns-established:
  - "gate_wizard and criteria_create follow the same handler shape as milestone_create and spec_create"

requirements-completed:
  - WIZM-01
  - CLIB-04

# Metrics
duration: 3min
completed: 2026-04-13
---

# Phase 68 Plan 01: MCP Surface Summary

**gate_wizard and criteria_create MCP tools wrapping apply_gate_wizard/apply_criteria_wizard via spawn_blocking, with GateWizardResponse and CriteriaCreateResponse structs and 6 integration tests**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-04-13T02:10:46Z
- **Completed:** 2026-04-13T02:13:13Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- gate_wizard MCP handler: creates or edits gates.toml from GateWizardInput, returns path + GatesSpec + warnings
- criteria_create MCP handler: creates criteria library TOML from CriteriaWizardInput, returns path + CriteriaLibrary + warnings
- Both tools reject duplicates (overwrite=false) returning isError=true via domain_error()
- Module doc comment updated with both new tool entries
- 201 total tests pass, no clippy warnings

## Task Commits

Tests and implementation were committed together due to pre-commit hook requiring compilation:

1. **Tasks 1+2: gate_wizard + criteria_create handlers + tests** - `2266d55` (feat)

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` - Added GateWizardResponse + CriteriaCreateResponse structs, gate_wizard + criteria_create handlers, 6 integration tests, updated module doc comment

## Decisions Made

- Tests and implementation committed in a single feat commit: the pre-commit hook runs `cargo clippy --workspace -D warnings`, which requires the implementation to exist for the test code to compile. Standard TDD RED/GREEN separation was verified by running the tests manually before adding the implementation.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

Formatting issue on first commit attempt: `#[tool(description = "...")]` needed multi-line formatting with closing `)]` on its own line to match rustfmt style. Fixed before final commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- gate_wizard and criteria_create are callable via MCP
- Phase 68 Plan 02 (criteria_list, criteria_get, spec_resolve) can proceed immediately — same server.rs file, same handler patterns
