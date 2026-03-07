---
created: 2026-03-04T10:00
title: EnforcementSummary fields lack #[serde(default)] attributes
area: assay-types
severity: important
files:
  - crates/assay-types/src/enforcement.rs:53-59
---

## Problem

`EnforcementSummary` struct derives `Default` but its public fields lack `#[serde(default)]` attributes. This causes serde to reject partial JSON during deserialization, preventing graceful handling of older schemas or optional fields.

## Solution

Add `#[serde(default)]` to all fields in `EnforcementSummary` to align with serde best practices and improve compatibility.
