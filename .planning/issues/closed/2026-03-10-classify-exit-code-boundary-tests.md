# Add Boundary Values to `classify_exit_code` Tests

## Description

`classify_exit_code` tests in `gate/mod.rs` do not cover boundary exit codes. Cases for -1, 128, and 255 should be added to ensure correct classification at the edges of the exit code range.

## File Reference

`crates/assay-core/src/gate/mod.rs` (tests)

## Category

tests
