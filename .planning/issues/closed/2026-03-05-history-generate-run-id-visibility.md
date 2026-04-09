---
created: 2026-03-05T00:00
title: generate_run_id should be pub(crate) not pub
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

`generate_run_id()` is public (`pub`), exposing an internal implementation detail. It's only used within the history module. Public APIs should be intentional; internal helpers should be restricted.

## Solution

Change `pub fn generate_run_id` to `pub(crate) fn generate_run_id` (or even `fn` if only used in this module). If callers outside `assay-core` need run ID generation, provide a stable public API that encapsulates the behavior.

