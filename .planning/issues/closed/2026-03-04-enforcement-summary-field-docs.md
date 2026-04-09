---
created: 2026-03-04T10:00
title: EnforcementSummary public fields lack doc comments
area: assay-types
severity: important
files:
  - crates/assay-types/src/enforcement.rs:54-58
---

## Problem

Public fields in `EnforcementSummary` have no doc comments, which is inconsistent with all other public types in the types crate and reduces API clarity.

## Solution

Add doc comments to all public fields in `EnforcementSummary` explaining their purpose and semantics.
