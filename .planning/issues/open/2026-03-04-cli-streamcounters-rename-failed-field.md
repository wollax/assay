---
created: 2026-03-04T00:00
title: Rename failed → total_failed in StreamCounters
area: assay-cli
provenance: github:wollax/assay#PR-review
files:
  - crates/assay-cli/src/main.rs
---

## Problem

The field name `failed` on `StreamCounters` is ambiguous when the struct also tracks `has_required_failure` and `has_advisory_failure`. Callers must reason about the relationship between `failed`, `required_failed`, and `advisory_failed` across different contexts.

## Solution

Rename the `failed` field to `total_failed` to make it explicit that it represents the total count of failures (required + advisory combined), disambiguating it from the boolean flags that track failure enforcement levels.
