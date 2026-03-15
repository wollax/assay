> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: Watcher temp file filter suffixes (.tmp, ~) should be documented or configurable
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/watcher.rs:74
---

## Problem

The watcher filters out files ending in `.tmp` and `~` as temp file artifacts, but these suffixes are hardcoded magic strings with no documentation explaining why they were chosen or what tools produce them.

## Solution

Add a comment explaining the rationale (e.g., atomic write patterns from editors/tools). Consider extracting the suffixes into a named constant or making them configurable via `GuardConfig` for environments with different temp file conventions.