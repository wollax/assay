---
created: 2026-03-05T00:00
title: Add Eq derive alongside PartialEq (no floats in types)
area: assay-types
provenance: phase-14-review
files:
  - crates/assay-types/src/gate_run.rs
  - crates/assay-types/src/gate.rs
---

## Problem

Structs in `gate_run.rs` and `gate.rs` derive `PartialEq` but not `Eq`. Since these types contain no floating-point fields, they can safely derive `Eq` (which guarantees reflexivity, symmetry, and transitivity). Not deriving `Eq` is unusual and may confuse readers.

## Solution

Add `Eq` derive to all structs that derive `PartialEq` and contain no floats:
- `GateRunRecord`
- Related structs in `gate.rs`

