---
created: 2026-03-04T10:00
title: Add deny_unknown_fields to QualityAttributes
area: assay-types
severity: suggestion
files:
  - crates/assay-types/src/feature_spec.rs:188
---

## Problem

`QualityAttributes` lacks `#[serde(deny_unknown_fields)]`, allowing silent acceptance of typos or deprecated fields. This reduces schema strictness and makes breaking changes harder to detect.

## Solution

Add `#[serde(deny_unknown_fields)]` to ensure users are notified of unrecognized fields, improving schema validation and UX.
