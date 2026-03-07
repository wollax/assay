---
created: 2026-03-01T04:30
title: Tighten deny.toml multiple-versions from warn to deny
area: tooling
provenance: github:wollax/assay#15
files:
  - deny.toml:20
---

## Problem

`multiple-versions = "warn"` allows duplicate crate versions (crossterm 0.28/0.29, thiserror 1/2) to pass CI silently. Having two major versions of crossterm in a TUI binary can cause subtle runtime divergences.

## Solution

Change to `multiple-versions = "deny"` and add explicit `[[bans.skip]]` entries with justification comments for known unavoidable duplicates (crossterm pin until ratatui aligns, thiserror transitive dep).
