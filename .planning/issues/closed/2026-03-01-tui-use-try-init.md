> **Closed:** 2026-03-15 — Won't fix. Superseded by v0.4.0 architecture (phases 35-44).


---
created: 2026-03-01T04:30
title: Use ratatui::try_init() instead of panicking init()
area: assay-tui
provenance: github:wollax/assay#12
files:
  - crates/assay-tui/src/main.rs:12
---

## Problem

`ratatui::init()` panics on failure (e.g., stdout not a TTY, CI environment, non-interactive shell) instead of returning a `Result`. Since `main()` already returns `color_eyre::Result<()>`, the error propagation channel exists but isn't used. Users see an opaque panic instead of a meaningful error like "assay-tui requires an interactive terminal."

## Solution

Replace `ratatui::init()` with `ratatui::try_init()?` to propagate terminal initialization errors through the existing Result path.