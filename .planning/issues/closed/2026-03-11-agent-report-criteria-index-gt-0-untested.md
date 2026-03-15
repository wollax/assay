# No test for multiple `AgentReport` criteria at index > 0

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

Tests for `AgentReport` criteria validation only exercise the first criterion (index 0). When a spec has multiple criteria, the validation and location logic for criteria at index 1 and beyond are never exercised. This could mask bugs in index-dependent span or location calculations.

## Suggested Fix

Add a test with an `AgentReport` spec that has two or more criteria entries, where an invalid criterion appears at index 1 (or later). Assert that the resulting diagnostic references the correct index.
