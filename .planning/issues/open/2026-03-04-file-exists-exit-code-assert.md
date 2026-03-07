---
created: 2026-03-04T10:00
title: Add exit_code assertion to evaluate_file_exists_missing test
area: assay-core
severity: important
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

`evaluate_file_exists_missing` test doesn't assert that `exit_code == None` when a file doesn't exist, leaving a gap in test coverage for missing file behavior.

## Solution

Add explicit assertion verifying exit code is None for file-not-found scenarios.
