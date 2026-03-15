> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: context_pct calculation duplicated between check_and_respond and re_evaluate_after_prune
area: assay-core
severity: important
files:
  - crates/assay-core/src/guard/daemon.rs:138
  - crates/assay-core/src/guard/daemon.rs:279
---

## Problem

The context percentage calculation (token estimate divided by available window minus overhead) is implemented independently in both `check_and_respond` and `re_evaluate_after_prune`. The two implementations have subtly different error handling paths and the `check_and_respond` version also has a file-size heuristic fallback that `re_evaluate_after_prune` lacks, making them easy to diverge.

## Solution

Extract the context percentage calculation into a shared helper method on `GuardDaemon` (or a free function) that both call sites use, ensuring consistent logic and error handling.