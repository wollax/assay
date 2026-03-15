---
created: 2026-03-09T21:00
title: Add test with multi-line input for truncation
area: core
provenance: github:wollax/assay#77
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

All truncation unit tests use single-line or repeated-char inputs. Real command output contains newlines. There is no test verifying that the head contains early lines and the tail contains late lines after truncation.

## Solution

Add a test with multi-line input (e.g., numbered lines) and assert that the head portion contains the first lines and the tail portion contains the last lines.
