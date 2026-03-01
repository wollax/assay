---
created: 2026-03-01T04:30
title: Establish error type hierarchy in assay-core
area: assay-core
provenance: github:wollax/assay#17
files:
  - crates/assay-core/src/lib.rs
---

## Problem

`thiserror` is declared as a dependency but no error types are defined. Empty `pub mod` stubs provide no error propagation contract. Each future contributor may make independent error handling choices, leading to inconsistency.

## Solution

This is already planned as FND-02 and FND-03 in Phase 3 (Error Types and Domain Model). Track here as a backlog reminder in case Phase 3 scope changes.
