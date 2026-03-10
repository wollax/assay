---
created: 2026-03-09T21:15
title: Missing test for cleanup with force=true on clean worktree
area: core
provenance: github:wollax/assay#96
files:
  - crates/assay-core/src/worktree.rs:279-283
---

## Problem

`cleanup` with `force=true` on a clean worktree is untested. The condition `if force` triggers `--force` even when clean. If refactored incorrectly, the behavior change would be undetected.

## Solution

Add an integration test: create worktree, cleanup with `force=true` (no dirty files), verify success.
