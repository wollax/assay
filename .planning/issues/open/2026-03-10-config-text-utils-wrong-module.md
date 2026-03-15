# Move Text Utilities Out of `config` Module

## Description

`TruncatedLine`, `translate_position`, and `truncate_source_line` are general-purpose text/display utilities but live in the `config` module. They would be more discoverable in a dedicated `crate::fmt` or `crate::display` utility module.

## File Reference

`crates/assay-core/src/config/mod.rs`

## Category

code
