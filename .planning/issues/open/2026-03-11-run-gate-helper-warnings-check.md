# run_gate test helper should check for unexpected warnings

**Area:** crates/assay-mcp/tests/mcp_handlers.rs
**Severity:** suggestion
**Source:** PR review (phase 35)

## Description

The `run_gate` test helper asserts `!result.is_error` but doesn't check for unexpected `warnings` in the response JSON. Adding a warnings-absent assertion would catch regressions where save failures go unnoticed.
