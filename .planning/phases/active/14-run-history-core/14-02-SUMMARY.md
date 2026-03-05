---
phase: 14
plan: "02"
title: "History persistence integration tests"
status: complete
started: "2026-03-05T15:12:38Z"
completed: "2026-03-05T16:20:26Z"
duration_minutes: 68
commits:
  - hash: "5c41069"
    message: "test(14-02): add concurrent save safety test"
  - hash: "fcd1d20"
    message: "test(14-02): add crash resilience and full-fidelity roundtrip tests"
---

Integration tests proving concurrent write safety, crash resilience, and full-fidelity roundtrip serialization for the history persistence module.

## Tasks

### Task 1: Concurrent save safety test

- **Status:** Complete
- **Commit:** `5c41069`
- **What:** Added `test_concurrent_saves_produce_distinct_files` — spawns 10 threads each saving a distinct GateRunRecord for the same spec, then verifies all 10 produce distinct files, `list()` returns 10 entries, and every entry deserializes correctly.
- **Proves:** HIST-04 (concurrent writes produce distinct files without corruption).

### Task 2: Crash resilience and full-fidelity roundtrip tests

- **Status:** Complete
- **Commit:** `fcd1d20`
- **What:** Added two tests:
  1. `test_partial_write_leaves_no_corrupt_file` — writes truncated JSON debris to the results directory, verifies `list()` ignores it, and confirms valid saves work alongside it.
  2. `test_full_fidelity_roundtrip` — builds a GateRunRecord with all fields populated (working_dir, multiple CriterionResults with varied enforcement levels, truncation fields, FileExists/Command gate kinds), saves and loads it, asserts full structural equality, and independently deserializes from raw JSON.
- **PartialEq additions:** Added `PartialEq` derive to `GateRunRecord`, `GateRunSummary`, `CriterionResult`, and `GateResult` (non-breaking additive change).

## Decisions

- `PartialEq` derived on four types (`GateRunRecord`, `GateRunSummary`, `CriterionResult`, `GateResult`) to enable structural equality in tests. All nested types (`GateKind`, `Enforcement`, `EnforcementSummary`) already had it.

## Deviations

None.

## Verification

- `just ready` passes (fmt-check, lint, test, deny).
- All history tests pass: unit tests from Plan 01 + integration tests from this plan.

## Success Criteria Coverage

| Criterion | Test | Status |
|---|---|---|
| 10 concurrent saves produce distinct files | `test_concurrent_saves_produce_distinct_files` | Proven |
| Partial write leaves no corrupt JSON | `test_partial_write_leaves_no_corrupt_file` | Proven |
| Full-fidelity roundtrip | `test_full_fidelity_roundtrip` | Proven |
| `just ready` passes | Full suite | Verified |
