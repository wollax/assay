> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: GuardStatus could implement Display for user-friendly output
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/mod.rs:79
---

## Problem

`GuardStatus` derives `Debug` but has no `Display` implementation. The CLI handler in `main.rs` manually formats the status with `println!` statements, duplicating the presentation logic outside the type.

## Solution

Implement `std::fmt::Display` for `GuardStatus` (e.g., "Running (PID 1234)" / "Stopped") so both the CLI and any future consumers can use `{status}` directly.