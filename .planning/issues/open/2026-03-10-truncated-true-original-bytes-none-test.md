---
created: 2026-03-10T13:50
title: Test truncated=true with original_bytes=None edge case
area: testing
provenance: local
files:
  - crates/assay-mcp/src/server.rs
---

## Problem

The type system allows `truncated: true` with `original_bytes: None`. This is a valid upstream state from `GateResult`, but no test covers this combination to verify MCP response serialization handles it correctly.

## Solution

Add a test case with `truncated: Some(true), original_bytes: None` and verify JSON output includes `"truncated": true` but omits `original_bytes`.
