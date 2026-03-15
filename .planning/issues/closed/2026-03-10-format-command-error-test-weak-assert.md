# Strengthen `format_command_error` Test Assertions

## Description

Tests for `format_command_error` in `gate/mod.rs` assert `msg.contains("cargo")`, which would pass even if the command name is not quoted in the output. The assertion should be `msg.contains("'cargo'")` to match the actual format and guard against regressions.

## File Reference

`crates/assay-core/src/gate/mod.rs` (tests)

## Category

tests
