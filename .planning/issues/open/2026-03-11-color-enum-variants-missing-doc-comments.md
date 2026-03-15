# `Color` enum variants lack individual doc comments

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

The `Color` enum (used in the DFS cycle-detection algorithm) has no per-variant doc comments. The meaning of each variant is non-obvious to readers unfamiliar with graph colouring algorithms: `White` means unvisited, `Gray` means the node is currently on the DFS path (in-progress), and `Black` means fully explored.

## Suggested Fix

Add doc comments to each variant:

```rust
/// Node has not been visited yet.
White,
/// Node is currently on the active DFS path (cycle detection in progress).
Gray,
/// Node has been fully explored with no cycle found.
Black,
```
