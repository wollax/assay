# Test: `map_evaluator_output` with empty criteria should assert zero counts

**Area:** crates/assay-core/src/evaluator.rs
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`map_record_has_valid_run_id_and_version` passes empty criteria to `map_evaluator_output` but never asserts that the resulting `EvaluatorSummary` has zero counts for `passed`, `failed`, `warned`, and `skipped`. Without these assertions, the test does not guard against counter initialisation bugs.

## Suggested Fix

Add assertions to the existing test:

```rust
assert_eq!(summary.passed, 0);
assert_eq!(summary.failed, 0);
assert_eq!(summary.warned, 0);
assert_eq!(summary.skipped, 0);
```

## Category

testing
