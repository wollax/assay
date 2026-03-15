# Extract `single_failing_summary` Test Helper

## Description

Tests covering the stdout fallback path for failure reasons repeat approximately 35 lines of boilerplate each. Extracting a `single_failing_summary` helper function would reduce duplication and make the intent of each test clearer.

## File Reference

`crates/assay-mcp/src/server.rs` (stdout fallback tests)

## Category

testing

## Severity

low
