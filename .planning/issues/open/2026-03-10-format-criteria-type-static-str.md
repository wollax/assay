# `format_criteria_type` Could Return `&'static str`

## Description

`format_criteria_type` currently allocates a `String` but always returns one of 4 fixed string literals. Changing the return type to `&'static str` would eliminate the allocation entirely.

## File Reference

`crates/assay-cli/src/commands/mod.rs`

## Category

code
