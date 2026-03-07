---
created: 2026-03-04T00:00
title: Add doc comments to StreamCounters fields
area: assay-cli
provenance: github:wollax/assay#PR-review
files:
  - crates/assay-cli/src/main.rs
---

## Problem

`StreamCounters` struct fields (`passed`, `failed`, `skipped`) lack documentation. This makes it unclear whether these are success/failure/skip counts, total counts, or something else, and how they relate to enforcement-level tracking (`has_required_failure`, `has_advisory_failure`).

## Solution

Add doc comments to all fields in `StreamCounters` that explain:
- What each counter represents (e.g., "Total number of criteria that passed")
- Whether they include/exclude particular enforcement levels
- How they interact with the boolean failure-tracking fields
