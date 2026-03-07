---
created: 2026-03-04T10:00
title: Review ScanResult.specs backward-compat field duplication
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/spec/mod.rs:64-73
---

## Problem

`ScanResult.specs` appears to be a backward-compat field that duplicates data already available elsewhere. This adds maintenance burden and potential inconsistency.

## Solution

Document the backward-compat purpose, verify it's no longer needed, and consider deprecating or removing it if clients have migrated.
