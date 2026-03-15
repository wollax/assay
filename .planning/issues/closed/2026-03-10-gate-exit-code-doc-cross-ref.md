# `gate_exit_code()` Doc Should Cross-Reference `gate_blocked()`

## Description

The doc comment for `gate_exit_code()` restates the blocking condition inline. Instead it should reference `gate_blocked()` directly (e.g. `/// Returns 1 if [`gate_blocked`] is true, otherwise 0.`) to avoid duplication and keep the two in sync.

## File Reference

`crates/assay-cli/src/commands/gate.rs`

## Category

docs
