---
created: 2026-03-16T15:30
title: parse_conflicts state machine fragile on unexpected blank lines
area: core
provenance: local
files:
  - crates/assay-core/src/merge.rs:118-168
---

## Problem

The `parse_conflicts` function uses a boolean `in_messages` flag that flips on the first blank line after the tree OID. If git emits a blank line before the conflict file-info section (possible in edge cases or future git versions), the parser enters `in_messages` prematurely and skips all stage-info lines.

## Solution

Make the parser more robust: either track whether any stage-info lines have been seen before accepting a blank as the section separator, or anchor on `CONFLICT (` prefix regardless of section position to be maximally defensive.
