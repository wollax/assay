---
created: 2026-03-09T20:32
title: WorktreeInfo and WorktreeStatus field duplication
area: types
provenance: github:wollax/assay#83
files:
  - crates/assay-types/src/worktree.rs:33-64
---

## Problem

`WorktreeInfo` and `WorktreeStatus` duplicate `spec_slug`, `path`, and `branch` fields. `WorktreeStatus` is conceptually a `WorktreeInfo` + runtime state. This duplication means changes to shared fields must be made in two places.

## Solution

Consider composition with `#[serde(flatten)]`:

```rust
pub struct WorktreeStatus {
    #[serde(flatten)]
    pub info: WorktreeInfo,
    pub head: String,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
}
```
