---
created: 2026-03-09T20:32
title: Worktree ahead/behind use platform-dependent usize
area: types
provenance: github:wollax/assay#84
files:
  - crates/assay-types/src/worktree.rs:60-62
---

## Problem

`ahead` and `behind` fields on `WorktreeStatus` use `usize`, which is platform-dependent (32-bit vs 64-bit). For serializable types, `usize` serde behavior depends on target architecture, affecting JSON schema correctness across platforms.

## Solution

Change to `u32` — commit counts will never exceed `u32` range and it provides consistent serialization.
