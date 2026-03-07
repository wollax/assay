---
created: 2026-03-05T00:00
title: GateRunRecord working_dir should be Option<PathBuf> not Option<String>
area: assay-types
provenance: phase-14-review
files:
  - crates/assay-types/src/gate_run.rs
---

## Problem

`GateRunRecord::working_dir` is typed as `Option<String>`, forcing callers to parse it back into a `PathBuf` when they need to use it as a path. This introduces unnecessary conversions and loses the type safety that `PathBuf` provides.

## Solution

Change `working_dir` to `Option<PathBuf>`. Ensure serde serialization/deserialization still works correctly (serde can handle `PathBuf` natively).

