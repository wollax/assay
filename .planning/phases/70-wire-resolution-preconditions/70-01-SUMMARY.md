---
phase: 70-wire-resolution-preconditions
plan: 01
subsystem: history
tags: [rust, serde, types, preconditions, history, jsonschema]

# Dependency graph
requires:
  - phase: 66-evaluation-integration-validation
    provides: GateRunRecord, last_gate_passed(), history persistence layer
provides:
  - GateRunRecord with precondition_blocked field (serde backward-compat)
  - PreconditionStatus::all_passed() ergonomic helper
  - last_gate_passed() returns Some(false) for precondition-blocked records
affects: [70-02-plan, 70-03-plan, 68-mcp-surface]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Option<bool> with serde(default, skip_serializing_if) for backward-compatible field additions"
    - "TDD: write failing test, implement minimally, verify green, commit"

key-files:
  created: []
  modified:
    - crates/assay-types/src/precondition.rs
    - crates/assay-types/src/gate_run.rs
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-run-record-schema.snap
    - crates/assay-core/src/history/mod.rs
    - crates/assay-core/src/gate/session.rs
    - crates/assay-core/src/gate/evidence.rs
    - crates/assay-core/src/history/analytics.rs
    - crates/assay-core/src/evaluator.rs
    - crates/assay-core/tests/analytics.rs
    - crates/assay-core/tests/merge_propose.rs
    - crates/assay-mcp/src/server.rs

key-decisions:
  - "precondition_blocked uses Option<bool> not bool to maintain backward-compat with existing history records (None = normal run)"
  - "all_passed() vacuous-truth: empty PreconditionStatus returns true (no preconditions = nothing failed)"
  - "last_gate_passed() checks precondition_blocked before enforcement counters — blocked run is not a pass even if counters are zeroed"

patterns-established:
  - "precondition_blocked: None on all normal GateRunRecord construction sites — callers saving blocked records set Some(true) directly"

requirements-completed: [PREC-03]

# Metrics
duration: 7min
completed: 2026-04-13
---

# Phase 70 Plan 01: Wire Resolution Preconditions - Foundation Types Summary

**GateRunRecord gains `precondition_blocked: Option<bool>` field, PreconditionStatus gains `all_passed()`, and `last_gate_passed()` returns `Some(false)` for blocked records — backward-compatible foundation for Plans 02 and 03.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-04-13T16:18:40Z
- **Completed:** 2026-04-13T16:25:24Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Added `PreconditionStatus::all_passed()` with vacuous-truth semantics (empty = all passed)
- Added `precondition_blocked: Option<bool>` to `GateRunRecord` with `serde(default, skip_serializing_if)` for full backward compatibility
- Updated `last_gate_passed()` to return `Some(false)` when latest record has `precondition_blocked == Some(true)`
- Updated schema snapshot to include new field
- Updated all 11 `GateRunRecord` struct literal construction sites across the workspace

## Task Commits

Each task was committed atomically:

1. **Task 1: Add PreconditionStatus::all_passed() and GateRunRecord::precondition_blocked** - `15089fd` (feat)
2. **Task 2: Update last_gate_passed() to handle precondition-blocked records** - `be7d738` (feat)

## Files Created/Modified
- `crates/assay-types/src/precondition.rs` - Added `all_passed()` method and 4 tests
- `crates/assay-types/src/gate_run.rs` - Added `precondition_blocked: Option<bool>` field and 3 tests
- `crates/assay-types/tests/snapshots/schema_snapshots__gate-run-record-schema.snap` - Updated with new field
- `crates/assay-core/src/history/mod.rs` - Updated `last_gate_passed()` logic, `save_run()`, 3 test helpers, added 2 new tests
- `crates/assay-core/src/gate/session.rs` - Two `GateRunRecord` struct literals updated
- `crates/assay-core/src/gate/evidence.rs` - Test helper struct literal updated
- `crates/assay-core/src/history/analytics.rs` - Test helper struct literal updated
- `crates/assay-core/src/evaluator.rs` - `GateRunRecord` construction updated
- `crates/assay-core/tests/analytics.rs` - Test helper struct literal updated
- `crates/assay-core/tests/merge_propose.rs` - Test helper struct literal updated
- `crates/assay-mcp/src/server.rs` - Test helper struct literal updated

## Decisions Made
- `precondition_blocked` uses `Option<bool>` (not `bool`) so old records without the field deserialize to `None` (backward-compat mandatory per project convention)
- `all_passed()` uses vacuous truth: empty `PreconditionStatus` returns `true` (no preconditions = nothing blocked)
- `last_gate_passed()` checks `precondition_blocked` before enforcement counters — a blocked run is not a pass, even though its `required_failed` counter is 0 (zeroed because gate never ran)
- All normal construction sites use `precondition_blocked: None`; callers saving blocked records construct `GateRunRecord` directly with `Some(true)` (Plan 02 pattern)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated all GateRunRecord struct literals workspace-wide**
- **Found during:** Task 1 (TDD GREEN phase)
- **Issue:** Adding a new required struct field breaks all existing construction sites (11 locations: session.rs, evaluator.rs, evidence.rs, analytics.rs, history/mod.rs test helpers, test files, mcp/server.rs)
- **Fix:** Added `precondition_blocked: None` to every `GateRunRecord { ... }` literal across the workspace
- **Files modified:** 9 files beyond the plan's specified 3
- **Verification:** `cargo check --workspace` passes, pre-commit hook (fmt + clippy) passes
- **Committed in:** `15089fd` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 - missing critical struct field initialization)
**Impact on plan:** Required for correctness — struct field update is non-optional in Rust. No scope creep.

## Issues Encountered
- Pre-commit hook caught formatting issues in test code (`cargo fmt` reformatted long chained method calls). Fixed before final commit.
- Clippy via pre-commit hook found `--all-targets` struct literal failures not visible with `--workspace` check alone (test files). Required second round of fixes.

## Next Phase Readiness
- `GateRunRecord.precondition_blocked` field available for Plan 02 (CLI) to record blocked runs
- `PreconditionStatus::all_passed()` available for Plan 02's gate run check logic
- `last_gate_passed()` semantics correct for Plan 03 (MCP) precondition queries
- All 321 assay-types tests pass, all 32 history tests pass, full workspace compiles

---
*Phase: 70-wire-resolution-preconditions*
*Completed: 2026-04-13*

## Self-Check: PASSED

- crates/assay-types/src/precondition.rs — FOUND
- crates/assay-types/src/gate_run.rs — FOUND
- crates/assay-core/src/history/mod.rs — FOUND
- .planning/phases/70-wire-resolution-preconditions/70-01-SUMMARY.md — FOUND
- Commit 15089fd (Task 1) — FOUND
- Commit be7d738 (Task 2) — FOUND
