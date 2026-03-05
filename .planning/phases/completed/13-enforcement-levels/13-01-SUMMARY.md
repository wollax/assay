---
phase: 13-enforcement-levels
plan: 01
subsystem: types
tags: [enforcement, serde, schemars, inventory]
dependency-graph:
  requires: [11-01, 11-02, 12-01]
  provides: [enforcement-types, gate-section, enforcement-summary, enforcement-fields]
  affects: [13-02, 13-03]
tech-stack:
  added: []
  patterns: [two-tier-enforcement-enum, option-for-input-concrete-for-output]
key-files:
  created:
    - crates/assay-types/src/enforcement.rs
  modified:
    - crates/assay-types/src/lib.rs
    - crates/assay-types/src/criterion.rs
    - crates/assay-types/src/gates_spec.rs
    - crates/assay-types/src/gate_run.rs
    - crates/assay-types/tests/schema_roundtrip.rs
    - crates/assay-types/tests/snapshots/ (7 snapshot files)
decisions:
  - "Enforcement enum uses Copy (two-variant fieldless, read frequently during evaluation)"
  - "Input types use Option<Enforcement> (None = inherit from gate section); output types use concrete Enforcement"
  - "GateSection uses deny_unknown_fields (user-authored input); EnforcementSummary does not (output type)"
  - "Schema snapshots accepted for all 7 affected types"
metrics:
  duration: ~8 minutes
  completed: 2026-03-04
---

# Phase 13 Plan 01: Enforcement Type Layer Summary

Complete enforcement type foundation with Enforcement enum (Required/Advisory), GateSection for spec-level defaults, and EnforcementSummary for result breakdown; all existing types updated with enforcement fields and backward compatibility preserved via serde(default).

## What Was Done

### Task 1: Create enforcement.rs and wire into lib.rs
- Created `crates/assay-types/src/enforcement.rs` with three types:
  - `Enforcement` enum: `Required` (default) / `Advisory`, kebab-case serde, Copy + Default
  - `GateSection` struct: `enforcement: Enforcement` with `deny_unknown_fields`
  - `EnforcementSummary` struct: four `usize` counters (required_passed/failed, advisory_passed/failed)
- All three types registered in schema registry via `inventory::submit!`
- Wired module into `lib.rs` with `pub mod enforcement` and re-exports
- Added `gate: Option<GateSection>` field to `Spec` struct (between description and criteria)
- Commit: `781963c`

### Task 2: Add enforcement fields to existing types
- `Criterion`: added `enforcement: Option<Enforcement>` after timeout
- `GateCriterion`: added `enforcement: Option<Enforcement>` after timeout (before requirements)
- `GatesSpec`: added `gate: Option<GateSection>` after description
- `CriterionResult`: added `enforcement: Enforcement` (resolved, always present, `serde(default)`)
- `GateRunSummary`: added `enforcement: EnforcementSummary` (`serde(default)`)
- Updated all struct literals in `schema_roundtrip.rs` (8 Criterion/Spec, 3 GateCriterion/GatesSpec, 4 CriterionResult, 2 GateRunSummary)
- Accepted 7 schema snapshot updates
- Commit: `6606c3d`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Schema snapshot updates required**
- **Found during:** Task 1 and Task 2
- **Issue:** Adding new fields to types changed their JSON schemas, causing insta snapshot tests to fail
- **Fix:** Accepted all pending schema snapshots (2 in Task 1, 7 in Task 2)
- **Files modified:** 7 snapshot files in `crates/assay-types/tests/snapshots/`
- **Commits:** `781963c`, `6606c3d`

## Verification Results

1. `cargo check -p assay-types` -- passes
2. `cargo test -p assay-types` -- 58 tests pass across 4 suites (unit, schema_roundtrip, schema_snapshots, doc-tests)
3. `Enforcement` serializes as `"required"` / `"advisory"` (kebab-case)
4. `Enforcement::default()` is `Required`
5. Backward compat confirmed: `gate_run_summary_backward_compat_deserialize` test passes (JSON without enforcement field)
6. `pub enforcement` fields present on all 5 types (Criterion, GateCriterion, CriterionResult, GateRunSummary, GateSection)

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Enforcement uses `Copy` trait | Two-variant fieldless enum read frequently during evaluation; Copy avoids unnecessary clones |
| Input types: `Option<Enforcement>` | `None` means "inherit from gate section default" -- resolution happens at evaluation time |
| Output types: concrete `Enforcement` | CriterionResult carries resolved enforcement (always Required or Advisory) |
| GateSection: `deny_unknown_fields` | User-authored input type; catches typos |
| EnforcementSummary: no `deny_unknown_fields` | Output type; follows STATE.md decision from 11-01 |

## Next Phase Readiness

Plan 02 (evaluation logic) can proceed. All types are in place:
- `resolve_enforcement()` helper can use `Option<Enforcement>` + `Option<GateSection>` -> `Enforcement`
- `evaluate_all()` / `evaluate_all_gates()` can populate `EnforcementSummary` on `GateRunSummary`
- `to_criterion()` in assay-core will need `enforcement` field added (noted in plan)
- assay-core struct literals (Spec, GateCriterion, etc.) need `gate: None` / `enforcement: None` added

No blockers.
