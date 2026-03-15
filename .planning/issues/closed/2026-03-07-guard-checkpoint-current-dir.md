> **Closed:** 2026-03-15 — Deferred. Out of scope for v0.4.0 tech debt sweep. Guard daemon issues form a coherent sub-sweep for a dedicated guard cleanup phase.


---
created: 2026-03-07T08:00
title: try_save_checkpoint uses std::env::current_dir() instead of stored project dir
area: assay-core
severity: important
files:
  - crates/assay-core/src/guard/daemon.rs:304
---

## Problem

`try_save_checkpoint` calls `std::env::current_dir()` to resolve the project directory, but the daemon runs as a long-lived background process whose working directory may differ from the project root. This makes checkpoint extraction fragile and could silently save checkpoints from the wrong directory.

## Solution

Derive the project directory from `self.assay_dir` (its parent) or store it explicitly in `GuardDaemon` at construction time, and pass that to `extract_team_state` instead of relying on the process working directory.