# Assert `caret_offset` in `truncate_source_line` Truncation Tests

## Description

`truncate_source_line` tests in `config/mod.rs` never assert `caret_offset` when truncation actually occurs. Tests should add assertions on the returned `caret_offset` to verify the caret is positioned correctly after truncation.

## File Reference

`crates/assay-core/src/config/mod.rs` (tests)

## Category

tests
