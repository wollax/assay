# Parameter Validation Tests Use `.unwrap()` Without Context

## Description

New parameter validation tests call `.err().unwrap()` without a message, making failures hard to diagnose. These should be changed to `.err().expect("...")` with a description of what was expected to fail.

## File Reference

`crates/assay-mcp/src/server.rs` (parameter validation tests)

## Category

testing

## Severity

low
