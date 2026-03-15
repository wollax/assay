# Test: echo binary may not be portable

**Source:** Phase 37 PR review (code-reviewer, test-reviewer)
**Area:** assay-core/spec
**File(s):** crates/assay-core/src/spec/validate.rs

## Description

`test_validate_commands_known_binary` relies on `echo` being present on `PATH` to verify that a known-good binary passes validation. While `echo` is ubiquitous on Unix, it is a shell built-in on some platforms and may not appear as a standalone executable on others (e.g., certain minimal CI environments or Windows). Using `echo` as the sentinel binary makes the test brittle and environment-dependent.

## Suggested Fix

Replace `echo` with `cargo` or `rustc`, both of which are guaranteed to be on `PATH` in any environment that can compile and run the test suite.
