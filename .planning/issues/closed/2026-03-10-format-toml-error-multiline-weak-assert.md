# Strengthen `format_toml_error_multiline` Test Assertion

## Description

`format_toml_error_multiline` in `config/mod.rs` tests asserts `contains("line")`, which is too broad. The assertion should be tightened to `contains("line 2")` and should also verify that the caret character is present in the output.

## File Reference

`crates/assay-core/src/config/mod.rs` (tests — `format_toml_error_multiline`)

## Category

tests
