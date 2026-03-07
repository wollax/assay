---
created: 2026-03-04T10:00
title: Always serialize GateRunSummary.results instead of using skip_serializing_if
area: assay-types
severity: important
files:
  - crates/assay-types/src/gate_run.rs:19
---

## Problem

`GateRunSummary.results` uses `#[serde(skip_serializing_if)]` on a core payload field. This breaks API consumers expecting consistent schema, and hiding empty results arrays makes it harder to distinguish "no results" from "undefined results".

## Solution

Remove `skip_serializing_if` and always serialize the results array, even when empty. This ensures API schema consistency.
