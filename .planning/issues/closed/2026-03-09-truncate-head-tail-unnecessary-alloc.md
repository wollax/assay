---
created: 2026-03-09T21:00
title: truncate_head_tail allocates on non-truncated path
area: core
provenance: github:wollax/assay#77
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

`truncate_head_tail` calls `input.to_string()` when no truncation is needed, allocating a new `String` even in the common case where the input is already within budget. This is wasteful when the caller already owns a `String`.

## Solution

Accept `String` by value instead of `&str` so the non-truncated path can return the owned string without cloning. Alternatively, return `Cow<str>` to avoid allocation when no truncation occurs.
