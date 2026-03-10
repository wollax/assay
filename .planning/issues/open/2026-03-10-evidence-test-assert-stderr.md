---
created: 2026-03-10T13:50
title: Assert stderr in evidence-independence truncation test
area: testing
provenance: local
files:
  - crates/assay-mcp/src/server.rs
---

## Problem

The `test_truncation_fields_independent_of_include_evidence` test checks `stdout.is_none()` / `stdout.is_some()` based on the evidence flag, but does not assert `stderr` follows the same pattern. If stdout and stderr evidence gating were accidentally decoupled, this test would not catch it.

## Solution

Add `stderr` assertions mirroring the existing `stdout` assertions in the evidence-independence test.
