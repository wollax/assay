# `GateHistoryParams` Missing-Name Case Not Tested

## Description

The `GateHistoryParams` missing-name case is not covered by tests, leaving a gap in MCP-01 coverage. A test that omits the `name` field should be added to verify the correct error is returned.

## File Reference

`crates/assay-mcp/src/server.rs` (`GateHistoryParams`)

## Category

testing

## Severity

medium
