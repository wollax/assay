---
created: 2026-03-09T21:15
title: ASSAY_WORKTREE_DIR env var not documented in CLI help
area: cli
provenance: github:wollax/assay#93
files:
  - crates/assay-core/src/worktree.rs:110
---

## Problem

`ASSAY_WORKTREE_DIR` environment variable is read in `resolve_worktree_dir` but not documented in CLI `--help` output or any user-facing surface. Users/agents won't know it exists unless they read the source.

## Solution

Add env var documentation to the `--worktree-dir` help text or to the `worktree` subcommand description.
