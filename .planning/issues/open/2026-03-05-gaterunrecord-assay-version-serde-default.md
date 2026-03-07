---
created: 2026-03-05T00:00
title: assay_version could use serde(default) for legacy resilience
area: assay-types
provenance: phase-14-review
files:
  - crates/assay-types/src/gate_run.rs
---

## Problem

The `assay_version` field in `GateRunRecord` has no `serde(default)` attribute. If older records don't have this field, deserialization will fail. This reduces forward-compatibility and makes schema evolution harder.

## Solution

Add `#[serde(default)]` to `assay_version` so that records missing this field can still be deserialized (using the default value as a fallback). This is a low-risk improvement to resilience.

