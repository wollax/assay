# Test: check_commands=true integration path untested

**Source:** Phase 37 PR review (test-reviewer)
**Area:** assay-core/spec
**File(s):** crates/assay-core/src/spec/validate.rs

## Description

The `check_commands: true` code path within `validate_spec` has no test coverage. All existing tests either omit command checking or use `check_commands: false`, so the branch that invokes command validation from the top-level entry point is never exercised. A bug introduced in how `validate_spec` wires `check_commands` into the subordinate validator would go undetected.

## Suggested Fix

Add an integration-style test that calls `validate_spec` with `check_commands: true` and verifies that command diagnostics are produced (or not) as expected.
