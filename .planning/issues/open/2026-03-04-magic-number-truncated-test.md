---
created: 2026-03-04T10:00
title: Document or extract 131072 magic number in truncated test
area: assay-types
severity: suggestion
files:
  - crates/assay-types/tests/schema_roundtrip.rs
---

## Problem

A hardcoded magic number `131_072` appears in truncated test without explanation. Future maintainers cannot determine if this value is correct or if it needs updating.

## Solution

Extract as a named constant with a comment explaining why this truncation threshold was chosen, or link to related documentation.
