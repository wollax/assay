# `validate_spec` never tested with `check_commands = true`

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

The test suite for `validate_spec` does not exercise the code path where `check_commands = true`. Command validation logic runs only when that flag is set, so its correctness is entirely unverified in the current test suite.

## Suggested Fix

Add tests that call `validate_spec` with `check_commands = true`, covering both valid commands (no diagnostics) and invalid/missing commands (error diagnostics emitted).
