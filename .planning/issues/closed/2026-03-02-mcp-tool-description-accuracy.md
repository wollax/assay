---
created: 2026-03-02T14:30
title: Tool descriptions have minor inaccuracies
area: mcp
provenance: github:wollax/assay#34
severity: important
files:
  - crates/assay-mcp/src/server.rs:117
  - crates/assay-mcp/src/server.rs:173
---

## Problem

Two tool description inaccuracies:

1. `spec_list` description says "Returns an array of {name, description, criteria_count} objects" but `description` is conditionally omitted when empty (`skip_serializing_if`). An agent would expect it always present.

2. `gate_run` description doesn't mention skipped criteria. The response includes `skipped` count and per-criterion `status: "skipped"` for criteria without commands, but the description only mentions "pass/fail status".

## Solution

1. Update spec_list: "Returns an array of {name, criteria_count, description?} objects where description is omitted when empty."
2. Update gate_run: Add "Criteria without a command are skipped and counted separately."
