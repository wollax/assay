# Pin MCP "No specs" Tests to Exact String

## Description

Three MCP tests in `server.rs` use `||` disjunction for the "no specs" response assertion. Since the test fixtures use empty directories, the response should always be `"No specs found"`. The assertions should be pinned to that exact string rather than allowing alternatives via `||`.

## File Reference

`crates/assay-core/src/server.rs` (tests)

## Category

tests
