---
created: 2026-03-05T00:00
title: load() doc comment describes deny_unknown_fields behavior that belongs on the type
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

The `load()` function doc comment describes `deny_unknown_fields` serde behavior. This is a type-level concern, not a function-level one, and the doc comment misdirects readers. If the attribute is removed (see issue #1), this doc becomes incorrect.

## Solution

Move any serialization/deserialization constraints to the `GateRunRecord` type's doc comment. Keep `load()`'s doc focused on its contract: what it reads, what errors it may return, what it returns on success.

