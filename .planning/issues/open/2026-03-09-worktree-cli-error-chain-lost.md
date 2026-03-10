---
created: 2026-03-09T20:32
title: CLI worktree handlers discard error source chain
area: cli
provenance: github:wollax/assay#80
files:
  - crates/assay-cli/src/commands/worktree.rs:127
---

## Problem

`anyhow::anyhow!("{e}")` discards the error source chain by formatting to string. This loses the full cause chain that `anyhow`'s debug formatting (`{e:#}`) would display. Pattern appears at multiple call sites in the worktree CLI handlers.

## Solution

Use `.map_err(|e| anyhow::Error::from(e))` or implement `From<AssayError> for anyhow::Error` to preserve the full chain.
