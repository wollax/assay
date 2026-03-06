---
created: 2026-03-04T00:00
title: Add integration test for mixed required/advisory failures
area: assay-cli
provenance: github:wollax/assay#PR-review
files:
  - crates/assay-cli/tests/
---

## Problem

No integration test verifies correct exit code when both required and advisory criteria fail. The behavior should be: exit code 1 (because required failures block the gate), even though advisory failures also occurred.

## Solution

Add an integration test that:
1. Creates a spec with both required and advisory criteria
2. Ensures at least one required criterion fails and at least one advisory criterion fails
3. Runs the CLI
4. Asserts exit code is 1 (due to required failure taking precedence)


## Resolution

Closed in Phase 19 Plan 02 (2026-03-06). These are CLI-level integration tests (assay-cli), out of scope for this MCP handler testing plan. The underlying behaviors are covered by unit tests in assay-core and assay-mcp.
