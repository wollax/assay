---
created: 2026-03-04T10:00
title: Enforcement check block duplicated in handle_gate_run_all and handle_gate_run
area: assay-cli
severity: important
files:
  - crates/assay-cli/src/main.rs:789-798
  - crates/assay-cli/src/main.rs:885-894
---

## Problem

The `before_failed` / `resolve_enforcement` block is duplicated verbatim in both `handle_gate_run_all` and `handle_gate_run` functions, violating DRY principle and creating maintenance overhead.

## Solution

Extract the shared enforcement check logic into a reusable helper function and call it from both sites.
