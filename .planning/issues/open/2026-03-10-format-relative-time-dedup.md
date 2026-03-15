# `format_relative_time` and `format_relative_timestamp` Are Near-Duplicates

## Description

`format_relative_time` and `format_relative_timestamp` share identical threshold logic; the only difference is that `format_relative_timestamp` first parses an ISO string before delegating. Extract the shared logic into a private inner function to eliminate the duplication.

## File Reference

`crates/assay-cli/src/commands/mod.rs`

## Category

code
