---
created: 2026-03-04T10:00
title: Document is_valid_req_id multi-segment areas behavior
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/spec/mod.rs:362-374
---

## Problem

`is_valid_req_id()` allows multi-segment areas (e.g., `area/sub`) but this behavior is undocumented and may be unintentional. Callers cannot determine if nested areas are valid or an edge case.

## Solution

Document whether multi-segment areas are supported, add examples, or clarify validation rules in code comments.
