---
created: 2026-03-04T10:00
title: Add #[serde(default)] to GatesSpec.criteria field
area: assay-types
severity: important
files:
  - crates/assay-types/src/gates_spec.rs:51
---

## Problem

`GatesSpec.criteria` lacks `#[serde(default)]`, creating inconsistent error handling between deserialization failures and validation. Missing criteria field causes parse errors instead of graceful defaults.

## Solution

Add `#[serde(default)]` to the `criteria` field to allow specs without criteria to deserialize successfully with empty vec, then apply validation rules consistently.
