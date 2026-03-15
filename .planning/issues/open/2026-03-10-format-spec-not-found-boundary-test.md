# Add Boundary Test for `format_spec_not_found` at Exactly 10 Items

## Description

`format_spec_not_found` tests in `spec/mod.rs` cover the 11-item case (above `max_inline`) but not the 10-item boundary (at `max_inline`). A test with exactly 10 items should be added to verify the threshold behaviour.

## File Reference

`crates/assay-core/src/spec/mod.rs` (tests)

## Category

tests
