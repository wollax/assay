---
created: 2026-03-04T10:00
title: GateRunSummary passed/failed fields semantically ambiguous with enforcement levels
area: assay-types
severity: important
files:
  - crates/assay-types/src/gate_run.rs:22-27
---

## Problem

`GateRunSummary::passed` and `failed` fields are semantically ambiguous now that `enforcement` (required vs. advisory) exists. The `failed` field could refer to either required failures only or total failures (required + advisory), creating confusion for consumers.

## Solution

Add doc comments clarifying that `failed` counts total failures regardless of enforcement level, and that advisory failures do not cause non-zero exit codes despite being included in the count.
