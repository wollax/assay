# Add test for gate_report warnings field omission

**Area:** crates/assay-mcp/tests/mcp_handlers.rs
**Severity:** suggestion
**Source:** PR review (phase 35)

## Description

`gate_run` and `gate_finalize` have tests verifying `warnings` is absent when empty (via `skip_serializing_if`). `gate_report` has the same field but no corresponding test. Add one for consistency.
