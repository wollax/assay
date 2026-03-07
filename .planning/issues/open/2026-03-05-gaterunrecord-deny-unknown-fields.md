---
created: 2026-03-05T00:00
title: GateRunRecord deny_unknown_fields contradicts forward-compatibility
area: assay-types
provenance: phase-14-review
files:
  - crates/assay-types/src/gate_run.rs
---

## Problem

`GateRunRecord` has `#[serde(deny_unknown_fields)]` which breaks forward-compatibility. When future versions add new fields to `GateRunRecord`, older versions that read the persisted records will fail to deserialize them. Project convention dictates that output types (types meant to be persisted or shared) should NOT use this attribute.

## Solution

Remove `#[serde(deny_unknown_fields)]` from `GateRunRecord` to allow graceful degradation when reading records from newer versions.

