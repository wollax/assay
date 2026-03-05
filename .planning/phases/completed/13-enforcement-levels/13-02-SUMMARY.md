---
phase: 13-enforcement-levels
plan: 02
status: complete
started: 2026-03-04T23:27:34Z
completed: 2026-03-04
duration: ~124min
commits:
  - hash: 6da66cf
    message: "feat(13-02): add resolve_enforcement() and enforcement-aware evaluation"
  - hash: a6432f8
    message: "feat(13-02): add at-least-one-required validation to validate() and validate_gates_spec()"
  - hash: 90dea9b
    message: "style(13-02): fix import order in gate_run.rs"
tests_added: 10
tests_total: 188
---

# 13-02 Summary: Enforcement Evaluation Logic

Implemented enforcement-aware gate evaluation and parse-time validation, making the required/advisory distinction from Plan 01 functional in gate behavior.

## Tasks Completed

### Task 1: resolve_enforcement() and enforcement-aware evaluation
- Added `resolve_enforcement()` helper with precedence: criterion override > gate section default > Required
- Updated `evaluate_all()` to track `EnforcementSummary` (required/advisory pass/fail counts)
- Updated `evaluate_all_gates()` with identical enforcement tracking
- Updated `to_criterion()` to propagate the enforcement field
- Set `CriterionResult.enforcement` to resolved value for every criterion
- Skipped criteria excluded from enforcement counts (only in skipped bucket)
- Updated all struct literals across `gate/mod.rs`, `spec/mod.rs`, `schema_roundtrip.rs`, and `server.rs` for Plan 01 type changes
- Added 5 tests: precedence, advisory failure tracking, skipped exclusion, gates tracking, mixed enforcement

### Task 2: At-least-one-required validation
- Added validation check to `validate()`: specs with zero executable required criteria are rejected
- Added identical check to `validate_gates_spec()`
- Resolution follows same precedence as evaluation (criterion > gate section > Required)
- Only executable criteria count (must have cmd or path)
- Updated existing test TOML fixtures to include `cmd = "true"` for validation compliance
- Added 5 tests: all_advisory rejected, required_override accepted, no_gate_section defaults, gates_spec rejection, descriptive-only required doesn't count

## Deviations

1. **assay-mcp struct literals (auto-fix blocker):** Plan 01 added enforcement fields to `CriterionResult` and `GateRunSummary`, but `crates/assay-mcp/src/server.rs` test code also had struct literals needing updates. Fixed alongside Task 1 to unblock workspace compilation.

2. **gate_run.rs import ordering (auto-fix):** `cargo fmt` reordered imports in `crates/assay-types/src/gate_run.rs` (from Plan 01). Committed as a separate style fix.

3. **Existing test fixtures updated:** Multiple test TOML strings and struct literals in `spec/mod.rs` used descriptive-only criteria (no cmd/path). Updated to include `cmd = "true"` to comply with the new at-least-one-required validation without weakening test coverage.

## Decisions

- Backward compatibility preserved: `passed`/`failed`/`skipped` count fields on `GateRunSummary` compute exactly as before (total pass/fail regardless of enforcement). `EnforcementSummary` is purely additive.
- Existing specs without `[gate]` section default to `Required` enforcement and pass validation when they have at least one executable criterion.
