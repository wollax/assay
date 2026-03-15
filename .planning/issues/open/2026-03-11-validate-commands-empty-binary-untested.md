# Test: empty command string path untested

**Source:** Phase 37 PR review (test-reviewer)
**Area:** assay-core/spec
**File(s):** crates/assay-core/src/spec/validate.rs

## Description

The branch in `validate_commands` that handles an empty `binary` string (i.e., `binary.is_empty()`) is never exercised by any test. This guard exists to produce a meaningful diagnostic rather than failing with a confusing OS-level error, but because it is untested, a refactor could silently remove it without any test failing.

## Suggested Fix

Add a test that passes a command with an empty binary string and asserts that the expected diagnostic (error or warning) is produced.
