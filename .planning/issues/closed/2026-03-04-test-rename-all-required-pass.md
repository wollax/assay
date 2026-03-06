---
created: 2026-03-04T10:00
title: Test name all_required_pass_advisory_failures_still_pass uses inconsistent style
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

The test name `all_required_pass_advisory_failures_still_pass` uses an inconsistent naming style compared to other tests in the suite, mixing subject/predicate order and verbosity.

## Solution

Rename to a more consistent style matching project conventions, such as `evaluate_all_required_pass_advisory_failures_still_pass` or `advisory_failures_do_not_block_when_required_pass`.


## Resolution

Closed as acknowledged in Phase 19 Plan 02 (2026-03-06). Test naming/structure suggestions are low-priority style preferences. Current naming is functional and consistent within each crate.
