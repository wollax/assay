---
created: 2026-03-02T14:30
title: MCP tool handlers have zero direct tests
area: testing
provenance: github:wollax/assay#35
severity: suggestion
files:
  - crates/assay-mcp/src/server.rs
---

## Problem

All 3 tool handler methods (`spec_list`, `spec_get`, `gate_run`) have no direct tests. Existing tests cover helper functions and formatting logic but not the glue code in the handlers: parameter destructuring, early-return branching, and JSON serialization within the tool body.

The E2E verification covered this manually, but there are no automated integration tests that exercise the tool methods directly or through JSON-RPC transport.

## Solution

Either:
1. Add composed helper-sequence tests that mirror what each tool handler does
2. Or add JSON-RPC transport integration tests that start the server and send messages via stdin/stdout
