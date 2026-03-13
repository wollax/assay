---
created: 2026-03-13T10:45
title: Add GrowthRate serialization round-trip test
area: types
provenance: local
files:
  - crates/assay-types/tests/context_types.rs
  - crates/assay-types/src/context.rs:354-362
---

## Problem

The `TokenEstimate` round-trip and snapshot tests in `context_types.rs` always set `growth_rate: None`. The `GrowthRate` struct's serialization and deserialization are never tested directly, so a serde misconfiguration on `GrowthRate` fields would not be caught.

## Solution

Add a test that constructs a `TokenEstimate` with `growth_rate: Some(GrowthRate { ... })`, serializes to JSON, verifies `growth_rate` key is present with correct fields, and deserializes back to verify round-trip.
