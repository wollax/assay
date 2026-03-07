---
created: 2026-03-01T05:30
title: Add truncation metadata fields to GateResult
area: assay-types
phase: 3
provenance: brainstorm:2026-02-28T23-16-brainstorm/deterministic-report.md
files:
  - crates/assay-types/src/lib.rs
---

## Problem

Gate commands can produce unbounded stdout/stderr. When output is truncated (by streaming byte budget in Phase 7), the agent needs to know that evidence was clipped so it can request full output or re-run with a higher budget.

## Solution

Add `truncated: bool` and `original_bytes: Option<u64>` fields to `GateResult`. Truncation is a fact about the evidence, not a presentation concern — it belongs on the DTO.

```rust
pub struct GateResult {
    // ...existing fields...
    pub truncated: bool,
    pub original_bytes: Option<u64>, // Pre-truncation size (None if not truncated)
}
```

Apply `#[serde(skip_serializing_if)]` so `truncated: false` and `original_bytes: None` don't appear in serialized JSON.

## Resolution

Resolved during v0.2.0 development. `truncated` and `original_bytes` fields exist on `GateResult`.
