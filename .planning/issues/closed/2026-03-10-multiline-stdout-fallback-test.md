# No Test for Multiline Stdout in Failure Reason Fallback

## Description

There is no test verifying behaviour when stdout contains multiple lines in the failure reason fallback path. A test should confirm that only the first non-empty line is used as the failure reason, not subsequent lines.

## File Reference

`crates/assay-mcp/src/server.rs` (failure reason stdout fallback)

## Category

testing

## Severity

medium
