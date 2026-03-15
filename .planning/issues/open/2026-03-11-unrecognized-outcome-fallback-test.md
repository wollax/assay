# Add test for unrecognized outcome value returning error

**Area:** crates/assay-mcp/tests/mcp_handlers.rs
**Severity:** suggestion
**Source:** PR review (phase 35)

## Description

The server now returns a domain_error for unrecognized `outcome` values. Add an integration test with `outcome=Some("garbage")` asserting it returns an error response, documenting the validation behavior.
