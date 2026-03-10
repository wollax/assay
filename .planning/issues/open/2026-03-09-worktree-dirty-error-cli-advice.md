---
created: 2026-03-09T20:32
title: WorktreeDirty error contains CLI-specific advice
area: core
provenance: github:wollax/assay#85
files:
  - crates/assay-core/src/error.rs:232
---

## Problem

`WorktreeDirty` error message says "use --force to override" which is CLI-specific advice. When this error surfaces through MCP (where `force` defaults to `true`), the message is confusing and irrelevant.

## Solution

Make the domain error message context-neutral (e.g., "worktree `{spec_slug}` has uncommitted changes") and let the CLI layer append the `--force` hint when displaying the error.
