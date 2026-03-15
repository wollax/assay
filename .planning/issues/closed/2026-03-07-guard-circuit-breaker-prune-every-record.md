> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: circuit_breaker prune_old called on every record_recovery — could batch
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/circuit_breaker.rs:47
---

## Problem

`prune_old()` is called at the start of every `record_recovery()` call, iterating the deque to remove expired entries. With typical `max_recoveries` values (3-5) the cost is negligible, but the pattern is unnecessary since pruning on `should_trip` or `recovery_count` would suffice.

## Solution

Consider deferring pruning to read operations (`should_trip`, `recovery_count`, `current_tier`) rather than write operations, or only pruning when the deque exceeds `max_recoveries`. This is a minor optimization that mainly improves code clarity about when pruning is actually needed.