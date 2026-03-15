# Simplify `format_spec_not_found` Intermediate Vec

## Description

`format_spec_not_found` in `spec/mod.rs` (lines 84–95) builds a `Vec<&str>` solely to call `join` on it. This intermediate allocation could be simplified, e.g. by building the string directly or using an iterator.

## File Reference

`crates/assay-core/src/spec/mod.rs` (lines 84–95)

## Category

code
