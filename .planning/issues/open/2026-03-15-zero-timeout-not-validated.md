# Zero timeout not validated in MCP server

**Area:** crates/assay-mcp/src/server.rs
**Severity:** Medium
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`timeout: Some(0)` is accepted without validation and produces `Duration::ZERO`, which causes an immediate timeout. There is no guard that rejects or clamps this to a sensible minimum, making it easy for callers to trigger silent failures by accidentally passing zero.

## Suggested Fix

Validate the timeout before constructing the `Duration`:

```rust
if secs == 0 {
    return Err(McpError::invalid_params("timeout must be greater than zero", None));
}
```

Alternatively, document that `0` means "no timeout" if that semantic is intentional, and branch accordingly.

## Category

validation
