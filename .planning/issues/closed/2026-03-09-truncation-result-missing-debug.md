---
created: 2026-03-09T21:00
title: TruncationResult missing Debug derive
area: core
provenance: github:wollax/assay#77
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

`TruncationResult` struct does not derive `Debug`. This makes it harder to inspect values during debugging and prevents using `assert_eq!` in tests (which requires `Debug` for error messages).

## Solution

Add `#[derive(Debug)]` to the `TruncationResult` struct.
