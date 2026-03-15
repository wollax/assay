# Test: `map_evaluator_output` with Warn outcome on Required criterion untested

**Area:** crates/assay-core/src/evaluator.rs
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

The enforcement summary path `(Required, Warn)` — a Warn outcome on a Required criterion — has no test coverage. This path increments `required_failed` (or equivalent), and its absence means a regression in that branch would go undetected.

## Suggested Fix

Add a test that passes a criterion result with `outcome: Warn` and `enforcement: Required` and asserts that `required_failed` (or `failed`) is incremented correctly in the returned `EvaluatorSummary`.

## Category

testing
