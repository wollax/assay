---
created: 2026-03-04T10:00
title: Test name evaluate_all_advisory_failure_does_not_block misleading — only tests counters
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

The test name `evaluate_all_advisory_failure_does_not_block` suggests comprehensive verification of blocking behavior, but the test only checks counter values, not actual exit code or blocking semantics.

## Solution

Rename to a more accurate name like `evaluate_all_advisory_failures_counted` or add comprehensive blocking assertions to match the current test name's promise.


## Resolution

Closed as acknowledged in Phase 19 Plan 02 (2026-03-06). Test naming/structure suggestions are low-priority style preferences. Current naming is functional and consistent within each crate.
