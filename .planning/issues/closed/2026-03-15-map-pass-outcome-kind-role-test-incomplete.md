# Test: `map_pass_has_agent_report_kind_and_independent_role` should also cover Fail and Warn outcomes

**Area:** crates/assay-core/src/evaluator.rs
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`map_pass_has_agent_report_kind_and_independent_role` only tests the Pass outcome path for `kind` and `role` fields. The Fail and Warn outcome paths may produce different `kind` or `role` values, and those are not covered. A regression in how kind/role are set for non-passing outcomes would not be caught.

## Suggested Fix

Add separate test cases (or parametrize the existing test) for `outcome: Fail` and `outcome: Warn`, asserting the expected `kind` and `role` values for each.

## Category

testing
