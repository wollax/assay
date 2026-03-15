> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: daemon.rs methods could use tracing::instrument for structured tracing
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/daemon.rs:125
  - crates/assay-core/src/guard/daemon.rs:190
  - crates/assay-core/src/guard/daemon.rs:229
---

## Problem

The daemon methods use manual `info!`/`warn!`/`error!` calls with `[guard]` prefixes instead of leveraging `tracing::instrument`. This misses structured span context (method name, arguments) and requires manually tagging each log line.

## Solution

Add `#[tracing::instrument(skip(self))]` to `check_and_respond`, `handle_soft_threshold`, `handle_hard_threshold`, and `re_evaluate_after_prune` to get automatic span creation and structured context.