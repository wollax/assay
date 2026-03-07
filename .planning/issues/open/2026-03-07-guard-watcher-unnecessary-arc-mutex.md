---
created: 2026-03-07T08:00
title: SessionWatcher uses Arc<Mutex<OsString>> for immutable target_name
area: assay-core
severity: important
files:
  - crates/assay-core/src/guard/watcher.rs:50
  - crates/assay-core/src/guard/watcher.rs:65
---

## Problem

`target_name` is wrapped in `Arc<Mutex<OsString>>` and locked on every file system event, but the value is never mutated after construction. This adds unnecessary synchronization overhead and lock contention in the hot path of the watcher callback.

## Solution

Clone the `OsString` directly into the closure with `move`, eliminating both the `Arc` and `Mutex`. The closure only needs an owned copy for comparison.
