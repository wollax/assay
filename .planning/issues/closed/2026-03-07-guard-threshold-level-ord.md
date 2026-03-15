> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: ThresholdLevel could implement Ord for natural comparison
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/thresholds.rs:7
---

## Problem

`ThresholdLevel` variants have a natural ordering (None < Soft < Hard) but only derive `PartialEq` and `Eq`. Callers that need to compare severity levels must use pattern matching instead of standard comparison operators.

## Solution

Add `PartialOrd` and `Ord` derives to `ThresholdLevel`. The variant declaration order (None, Soft, Hard) already matches the desired ordering, so the derived implementation is correct.