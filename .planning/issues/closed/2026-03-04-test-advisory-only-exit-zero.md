---
created: 2026-03-04T00:00
title: Add integration test for advisory-only failures exit code 0
area: assay-cli
provenance: github:wollax/assay#PR-review
files:
  - crates/assay-cli/tests/
---

## Problem

No integration test verifies that a spec with only advisory failures exits with code 0. This is a critical behavior: advisory failures should not block the gate or fail the process.

## Solution

Add an integration test that:
1. Creates or loads a spec with only advisory criteria
2. Ensures at least one advisory criterion fails
3. Runs the CLI via both streaming and JSON output paths
4. Asserts exit code is 0 in both cases


## Resolution

Closed in Phase 19 Plan 02 (2026-03-06). These are CLI-level integration tests (assay-cli), out of scope for this MCP handler testing plan. The underlying behaviors are covered by unit tests in assay-core and assay-mcp.
