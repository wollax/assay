---
created: 2026-03-16T16:45
title: Rename char_limit to byte_limit in format_gate_evidence
area: assay-core
provenance: local
files:
  - crates/assay-core/src/gate/evidence.rs:30
---

## Problem

The `format_gate_evidence()` parameter is named `char_limit` but the implementation uses `.len()` (byte count), not `.chars().count()`. The name contradicts the actual behavior and could mislead callers into passing a character count instead of a byte count. A test (`truncation_enforces_byte_limit`) explicitly calls out this distinction.

## Solution

Rename `char_limit` to `byte_limit` across the function signature, all callers, and tests. Pure rename — no behavior change.
