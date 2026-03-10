---
created: 2026-03-10T13:50
title: Clarify original_bytes doc as sum of stdout + stderr
area: mcp
provenance: local
files:
  - crates/assay-mcp/src/server.rs:372-375
---

## Problem

The `original_bytes` doc comment on `CriterionSummary` says "Original combined byte count before truncation" but doesn't clarify it's the sum of both stdout and stderr streams. An agent consumer could infer it's a per-stream value.

## Solution

Update doc comment to: "Combined byte count of stdout and stderr before truncation."
