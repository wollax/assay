---
created: 2026-03-09T21:15
title: Worktree path uses to_string_lossy which corrupts non-UTF-8 paths
area: core
provenance: github:wollax/assay#87
files:
  - crates/assay-core/src/worktree.rs:164
  - crates/assay-core/src/worktree.rs:278
---

## Problem

`worktree_path.to_string_lossy().to_string()` replaces non-UTF-8 bytes with the Unicode replacement character. On Linux with certain locale configs, paths can contain non-UTF-8 sequences, causing git to receive a corrupted path string.

## Solution

Use `OsStr`-based command arguments via `Command::arg()` with `AsRef<OsStr>` instead of converting to `String`.
