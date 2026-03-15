# Test: Directory variant never tested in validate_spec

**Source:** Phase 37 PR review (test-reviewer)
**Area:** assay-core/spec
**File(s):** crates/assay-core/src/spec/validate.rs

## Description

`validate_spec` is only exercised with the `Legacy` spec variant in existing tests. The `Directory` path through `validate_spec` is completely untested, meaning any regressions or bugs specific to that branch would go undetected. This is a meaningful coverage gap given the two variants have different loading and structural characteristics.

## Suggested Fix

Add a test that constructs a `Directory`-variant spec and passes it through `validate_spec`, asserting both the valid and invalid cases.
