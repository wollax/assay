---
created: 2026-03-04T10:00
title: Add test for format_gate_response with empty results vector
area: assay-mcp
severity: important
files:
  - crates/assay-mcp/src/server.rs
---

## Problem

No test covers `format_gate_response` behavior when results vector is empty. Edge case handling for empty responses is untested.

## Solution

Add test case verifying correct formatting of gate responses with zero results.
