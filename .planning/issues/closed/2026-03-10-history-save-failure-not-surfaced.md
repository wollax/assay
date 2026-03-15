# History Save Failure Not Surfaced to MCP Caller

## Description

In the `gate_run` handler, a history save failure is logged but not surfaced to the MCP caller. The caller has no way to know the run record was not persisted. This is a pre-existing pattern but should be revisited to determine whether silent failure is acceptable or if the error should be propagated.

## File Reference

`crates/assay-mcp/src/server.rs` (`gate_run` handler)

## Category

error-handling

## Severity

medium
