> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: CircuitBreaker has no explicit reset() for post-trip recovery
area: assay-core
severity: important
files:
  - crates/assay-core/src/guard/circuit_breaker.rs:89
---

## Problem

The only way to un-trip the circuit breaker is via `reset_if_quiet()`, which requires the entire recovery window to have elapsed with no entries. There is no explicit `reset()` method for external callers (e.g., an operator command or config reload) to forcibly clear the tripped state. Once tripped, the daemon exits and cannot be recovered without restarting.

## Solution

Add a public `reset()` method that unconditionally clears the `tripped` flag and the recovery deque, enabling administrative recovery without requiring a full daemon restart.