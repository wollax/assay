# `EvaluatorCriterionResult` and `EvaluatorSummary` missing `Hash` derive

**Area:** crates/assay-types/src/evaluator.rs
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`EvaluatorCriterionResult` and `EvaluatorSummary` both derive `Eq` without `Hash`. For types that derive `Eq`, omitting `Hash` prevents their use as map keys or set members and is inconsistent with the rest of the evaluator type family.

## Suggested Fix

Add `Hash` to the derive lists for both types. Note that `Hash` requires all fields to implement `Hash`; any `f32`/`f64` fields would need special handling or the derive deferred.

## Category

types
