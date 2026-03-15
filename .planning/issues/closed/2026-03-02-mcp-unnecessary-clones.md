---
created: 2026-03-02T14:30
title: Unnecessary clone intermediaries in gate_run
area: mcp
provenance: github:wollax/assay#36
severity: important
files:
  - crates/assay-mcp/src/server.rs:194-195
---

## Problem

`spec_owned` and `working_dir_owned` clones are unnecessary. `spec` and `working_dir` are local variables that can be moved directly into the `spawn_blocking` closure. The `_owned` aliases add no semantic clarity and imply a subtlety that isn't there.

## Solution

Move `spec` and `working_dir` directly into the closure:
```rust
let summary = tokio::task::spawn_blocking(move || {
    assay_core::gate::evaluate_all(&spec, &working_dir, None, config_timeout)
})
```
