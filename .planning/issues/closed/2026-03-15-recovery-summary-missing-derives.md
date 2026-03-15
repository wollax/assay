# `RecoverySummary` Is Missing `Clone` and `PartialEq` Derives

## Description

`RecoverySummary` only derives `Debug`. All of its fields are `usize`, so `Clone` and `PartialEq` are free to derive and are needed to write clean test assertions (e.g., `assert_eq!(summary, expected)`) without manually unpacking each field.

## File Reference

`crates/assay-core/src/work_session.rs` — `RecoverySummary`

## Category

ergonomics / testing
