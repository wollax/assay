> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: Add Debug derive to GuardDaemon struct
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/daemon.rs:20
---

## Problem

`GuardDaemon` does not derive `Debug`, making it difficult to inspect daemon state during development or in error messages.

## Solution

Add `#[derive(Debug)]` to `GuardDaemon`. The `CircuitBreaker` field will also need a `Debug` derive (or manual implementation) to satisfy the bound.