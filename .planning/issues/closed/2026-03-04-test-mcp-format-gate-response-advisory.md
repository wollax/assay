---
created: 2026-03-04T00:00
title: Add unit test for format_gate_response with advisory criteria
area: assay-mcp
provenance: github:wollax/assay#PR-review
files:
  - crates/assay-mcp/src/server.rs
---

## Problem

No unit test covers the `format_gate_response` function when advisory criteria are present. The response must correctly populate the `enforcement` field and `advisory_failed` count to properly communicate enforcement-level information to MCP clients.

## Solution

Add a unit test that:
1. Constructs a `GateRunSummary` that includes both required and advisory criteria failures
2. Calls `format_gate_response` on it
3. Asserts that the `enforcement` field is correctly set (e.g., "mixed" or appropriately categorized)
4. Asserts that `advisory_failed` count matches the number of failed advisory criteria
5. Verifies the response structure is valid and all counts sum correctly


## Resolution

Resolved in Phase 19 Plan 02 (2026-03-06). Tests added in the appropriate crate test modules.
