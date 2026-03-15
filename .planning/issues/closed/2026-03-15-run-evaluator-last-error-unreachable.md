# `run_evaluator` last_error fallback should use `unreachable!()`

**Area:** crates/assay-core/src/evaluator.rs:400
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

The fallback branch in `run_evaluator` has a comment saying "Should be unreachable" but returns `EvaluatorError::NotInstalled`. If the branch is truly unreachable, using `unreachable!()` would make the invariant explicit and panic with a diagnostic message in debug builds instead of silently returning a misleading error variant.

## Suggested Fix

Replace the fallback with:

```rust
unreachable!("run_evaluator exhausted retries with no last_error set")
```

If the branch is reachable in some edge case, document it and pick a semantically correct error variant.

## Category

correctness
