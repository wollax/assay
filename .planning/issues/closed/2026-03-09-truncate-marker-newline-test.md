---
created: 2026-03-09T21:00
title: Add test for newline delimiters in truncation marker
area: core
provenance: github:wollax/assay#77
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

No test verifies the `\n[truncated: ...]\n` newline formatting of the truncation marker. If the marker format changes (e.g., missing newlines), no test would catch the regression.

## Solution

Add a test that asserts the truncation marker is surrounded by newline delimiters in the output.
