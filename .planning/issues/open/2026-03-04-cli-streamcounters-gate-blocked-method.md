---
created: 2026-03-04T00:00
title: Add gate_blocked() method to StreamCounters
area: assay-cli
provenance: github:wollax/assay#PR-review
files:
  - crates/assay-cli/src/main.rs
---

## Problem

The decision to block on exit code currently computes `counters.has_required_failure` inline in the caller. This logic should be encapsulated as a method on `StreamCounters` to improve readability and enable consistent reuse across multiple call sites.

## Solution

Add a `gate_blocked()` method to `StreamCounters` that returns `self.has_required_failure`, clearly expressing the intent that a required failure blocks the gate from proceeding.
