# Derive `Hash` on `CommandErrorKind`

## Description

`CommandErrorKind` in `gate/mod.rs` derives `PartialEq + Eq` but not `Hash`. Adding `Hash` would allow it to be used as a map or set key without requiring a wrapper.

## File Reference

`crates/assay-core/src/gate/mod.rs` (`CommandErrorKind`)

## Category

types
