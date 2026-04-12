---
phase: 66-evaluation-integration-validation
plan: "01"
subsystem: types
tags: [rust, serde, schemars, inventory, gate-evaluation, preconditions, history]

# Dependency graph
requires:
  - phase: 65-resolution-core
    provides: CriterionSource enum, ResolvedGate, ResolvedCriterion types
  - phase: 64-type-foundation
    provides: PreconditionStatus, SpecPreconditions types
provides:
  - GateEvalOutcome enum (Evaluated/PreconditionFailed) for precondition-aware evaluation
  - CriterionResult.source field for per-criterion provenance tracking
  - last_gate_passed() history helper for precondition checking
affects: [66-02-precondition-checking, 66-03-validation-diagnostics]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - internally-tagged serde enum using "outcome" discriminator field
    - backward-compat field addition with #[serde(default, skip_serializing_if)]
    - Option<bool> return convention for history queries (None = no history)

key-files:
  created: []
  modified:
    - crates/assay-types/src/gate_run.rs
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/history/mod.rs

key-decisions:
  - "GateEvalOutcome uses internally tagged serde (tag = \"outcome\") with snake_case rename: produces \"evaluated\" and \"precondition_failed\" discriminators"
  - "PreconditionFailed outcomes are NOT stored in run history per research recommendation — GateEvalOutcome is in-memory only"
  - "last_gate_passed() returns None (not Some(false)) for missing history, callers use .unwrap_or(false)"

patterns-established:
  - "Option<CriterionSource> field pattern: #[serde(default, skip_serializing_if = \"Option::is_none\")] for backward-compatible field addition"

requirements-completed: [PREC-03]

# Metrics
duration: 8min
completed: 2026-04-12
---

# Phase 66 Plan 01: Foundation Types Summary

**GateEvalOutcome enum with Evaluated/PreconditionFailed variants, CriterionResult.source provenance field, and last_gate_passed() history helper for precondition checking**

## Performance

- **Duration:** 8 min
- **Started:** 2026-04-12T00:02:33Z
- **Completed:** 2026-04-12T00:10:00Z
- **Tasks:** 2
- **Files modified:** 19 (including 71 CriterionResult struct literal updates across the workspace)

## Accomplishments
- GateEvalOutcome enum with Evaluated(GateRunSummary) and PreconditionFailed(PreconditionStatus) variants using internally tagged serde
- CriterionResult.source: Option<CriterionSource> field with full backward compatibility (old JSON without source field deserializes cleanly)
- last_gate_passed() helper returning Option<bool> from the latest gate run record's required_failed count
- GateEvalOutcome registered in schema registry via inventory as "gate-eval-outcome"
- 4 schema snapshots updated to reflect CriterionResult's new field

## Task Commits

Each task was committed atomically:

1. **Task 1: GateEvalOutcome enum and CriterionResult.source field** - `3dfce45` (feat)
2. **Task 2: last_gate_passed() history helper** - `3b8f48f` (feat)

## Files Created/Modified
- `crates/assay-types/src/gate_run.rs` - Added GateEvalOutcome enum, CriterionResult.source field, 7 TDD tests, schema registry entry
- `crates/assay-types/src/lib.rs` - Exported GateEvalOutcome in public API
- `crates/assay-core/src/history/mod.rs` - Added last_gate_passed() function and 4 TDD tests
- `crates/assay-core/src/gate/mod.rs` - Updated 7 CriterionResult literals with source: None
- `crates/assay-core/src/gate/session.rs` - Updated 5 CriterionResult literals with source: None
- `crates/assay-core/src/evaluator.rs` - Updated 2 CriterionResult literals with source: None
- `crates/assay-core/src/gate/evidence.rs` - Updated ~20 CriterionResult literals with source: None
- `crates/assay-core/src/review/mod.rs` - Updated 6 CriterionResult literals with source: None
- `crates/assay-mcp/src/server.rs` - Updated ~12 CriterionResult literals with source: None
- `crates/assay-types/tests/schema_roundtrip.rs` - Updated 4 CriterionResult literals with source: None
- `crates/assay-core/tests/analytics.rs` - Updated 1 CriterionResult literal with source: None
- `crates/assay-core/tests/merge_propose.rs` - Updated 1 CriterionResult literal with source: None
- `crates/assay-core/src/history/analytics.rs` - Updated 2 CriterionResult literals with source: None
- 4 schema snapshot `.snap` files updated

## Decisions Made
- GateEvalOutcome uses internally tagged serde representation with "outcome" as discriminator — stable JSON shape where tag appears alongside data, not nested
- PreconditionFailed outcomes are not stored in run history — callers save only the Evaluated(summary) inner value
- last_gate_passed() returns None for missing or empty history (callers .unwrap_or(false))

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated all existing CriterionResult struct literals with source: None**
- **Found during:** Task 1 (GateEvalOutcome enum and CriterionResult.source field)
- **Issue:** Adding a new required field to CriterionResult broke all existing struct literal constructions across 10 files (~71 occurrences)
- **Fix:** Used Python script to add source: None to all CriterionResult struct literals, then manually corrected 4 cases where the script misidentified EvaluatorCriterionResult or function-return patterns
- **Files modified:** 10 source files, 1 test file
- **Verification:** rtk cargo test -p assay-types -p assay-core — 1244 tests passing
- **Committed in:** 3dfce45 (Task 1 commit)

**2. [Rule 2 - Missing Critical] Updated 4 schema snapshots (insta)**
- **Found during:** Task 1 verification
- **Issue:** CriterionResult schema changed (new source field), existing snapshots were stale
- **Fix:** Ran INSTA_UPDATE=always cargo test to accept updated snapshots
- **Files modified:** criterion-result-schema.snap, gate-eval-context-schema.snap, gate-run-record-schema.snap, gate-run-summary-schema.snap
- **Verification:** All 71 schema snapshot tests pass
- **Committed in:** 3dfce45 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 2 — necessary consequences of the planned field addition)
**Impact on plan:** Both auto-fixes are direct consequences of the planned CriterionResult.source addition. No scope creep.

## Issues Encountered
- Python brace-counting script misidentified EvaluatorCriterionResult struct literals (similar name ends with "CriterionResult {") — required manual cleanup of 9 spurious source: None additions in evaluator.rs and 1 in server.rs
- Script also misidentified function-returning CriterionResult bodies in evidence.rs (closing brace of struct was mistaken for function closing brace) — required 5 manual fixes

## Next Phase Readiness
- GateEvalOutcome, CriterionResult.source, and last_gate_passed() are all ready for Plan 02 (precondition checking + resolved evaluation)
- No blockers

---
*Phase: 66-evaluation-integration-validation*
*Completed: 2026-04-12*
