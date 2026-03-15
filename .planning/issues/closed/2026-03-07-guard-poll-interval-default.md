> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: GuardConfig default poll_interval_secs=5 may be too aggressive for large sessions
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/config.rs:1
---

## Problem

The default `poll_interval_secs` of 5 seconds means the daemon performs a full token estimation (including file I/O and parsing) every 5 seconds. For large session files this could add non-trivial CPU and I/O load, especially combined with the file system watcher which already provides reactive checks.

## Solution

Consider increasing the default to 15 or 30 seconds since the file watcher provides sub-second reactive detection for active writes. The poll interval mainly serves as a safety net for missed events. Document the trade-off in the config field's doc comment.