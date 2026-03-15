# `EvaluatorCriterionResult.name` doc overstates constraint

**Area:** crates/assay-types/src/evaluator.rs:36
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

The doc comment on `EvaluatorCriterionResult.name` says the value "must match" the corresponding criterion name in the spec. In practice the code falls back to `Required` enforcement when the name does not match, so the constraint is not enforced. The doc comment overstates the guarantee and may mislead future implementors.

## Suggested Fix

Change "must match" to "should match":

```rust
/// The criterion name. Should match the criterion name defined in the spec;
/// if it does not, enforcement falls back to Required.
pub name: String,
```

## Category

documentation
