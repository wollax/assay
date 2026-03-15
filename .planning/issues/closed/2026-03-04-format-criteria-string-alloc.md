---
created: 2026-03-04T10:00
title: Avoid String allocation in format_criteria_type for static literals
area: assay-cli
severity: suggestion
files:
  - crates/assay-cli/src/main.rs:198-210
---

## Problem

`format_criteria_type()` allocates a String for each call to return static literal values. This creates unnecessary allocations for values that never change.

## Solution

Return `&'static str` or use `Cow<'static, str>` to avoid allocations for static literal returns.
