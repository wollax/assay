---
created: 2026-03-02T14:30
title: Response structs lack field-level documentation
area: mcp
provenance: github:wollax/assay#32
severity: suggestion
files:
  - crates/assay-mcp/src/server.rs:55-90
---

## Problem

`GateRunResponse`, `CriterionSummary`, and `SpecListEntry` have no field-level doc comments. Key undocumented behaviors:
- `CriterionSummary.reason` is the first non-empty stderr line (not full stderr)
- `SpecListEntry.description` is conditionally omitted when empty
- `CriterionSummary.status` is an untyped String ("passed"/"failed"/"skipped") rather than an enum

Also, `resolve_working_dir` comment says "matching CLI behavior" which will rot if CLI changes independently.

## Solution

Add field-level doc comments explaining non-obvious behaviors. Consider a `CriterionStatus` enum for compile-time guarantees on valid status values.

## Resolution

Resolved during Phase 17. All response structs have field-level doc comments.
