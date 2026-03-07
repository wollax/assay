---
created: 2026-03-07T08:00
title: handle_soft/hard_threshold silently swallow prune errors
area: assay-core
severity: important
files:
  - crates/assay-core/src/guard/daemon.rs:221
  - crates/assay-core/src/guard/daemon.rs:266
---

## Problem

Both `handle_soft_threshold` and `handle_hard_threshold` log prune errors but return `Ok(())` regardless. A failed prune means the session is still over threshold, yet the circuit breaker has already recorded a recovery attempt. This creates a silent failure mode where repeated prune failures consume recovery budget without actually reducing context size.

## Solution

Propagate prune errors (or at least a distinct warning-level error) so the caller can decide whether to escalate. Alternatively, avoid counting a recovery in the circuit breaker when the prune itself fails.
