---
created: 2026-03-04T10:00
title: GateSection arguably belongs in a config module, not enforcement.rs
area: assay-types
severity: suggestion
files:
  - crates/assay-types/src/enforcement.rs
---

## Problem

`GateSection` is a configuration construct that controls gate execution behavior, but it's currently defined in `enforcement.rs` alongside `Enforcement` and `EnforcementSummary`. This creates conceptual confusion about module purpose.

## Solution

Consider moving `GateSection` to a new `config` module or `gates_config` module to better reflect its role as a gate execution configuration, improving code organization and clarity.
