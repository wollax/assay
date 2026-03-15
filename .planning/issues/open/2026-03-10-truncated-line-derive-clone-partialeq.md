# Derive `Clone` and `PartialEq` on `TruncatedLine`

## Description

`TruncatedLine` in `config/mod.rs` does not derive `Clone` or `PartialEq`. Adding both would enable cleaner test assertions (equality comparisons) and make the type easier to use in contexts that require cloning.

## File Reference

`crates/assay-core/src/config/mod.rs` (`TruncatedLine`)

## Category

types
