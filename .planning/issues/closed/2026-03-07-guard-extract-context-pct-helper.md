> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: Extract context percentage calculation into a shared helper method
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/daemon.rs:138
  - crates/assay-core/src/guard/daemon.rs:279
---

## Problem

The token-to-percentage conversion logic (available window calculation, division, fallback handling) is repeated in `check_and_respond` and `re_evaluate_after_prune` with slight variations.

## Solution

Extract a `fn context_percentage(&self) -> crate::Result<f64>` method on `GuardDaemon` that encapsulates the token estimation and percentage calculation, and call it from both sites.