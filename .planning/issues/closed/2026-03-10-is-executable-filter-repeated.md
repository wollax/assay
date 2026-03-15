# `is_executable` Filter Repeated 3 Times

## Description

The predicate `c.cmd.is_some() || c.path.is_some()` appears twice in `gate.rs` and once in `spec.rs`. This logic should be extracted into a helper function or a method on `Criterion` to avoid the duplication and make the intent explicit.

## File Reference

- `crates/assay-cli/src/commands/gate.rs`
- `crates/assay-cli/src/commands/spec.rs`

## Category

code
