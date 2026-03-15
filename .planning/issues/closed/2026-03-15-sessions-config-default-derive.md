# `SessionsConfig` Should Derive `Default` Instead of Repeating the Magic Number

## Description

`SessionsConfig` has a hardcoded `3600` in at least one call site rather than using a `Default` impl. Deriving (or implementing) `Default` would centralise the default value, remove the magic number from call sites, and make the type easier to use in tests.

## File Reference

`crates/assay-types/src/lib.rs` — `SessionsConfig`

## Category

ergonomics / maintainability
