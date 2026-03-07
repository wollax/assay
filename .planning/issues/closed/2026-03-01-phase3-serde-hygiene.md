---
created: 2026-03-01T05:30
title: Apply serde skip_serializing_if on all domain types
area: assay-types
phase: 3
provenance: brainstorm:2026-02-28T23-16-brainstorm/deterministic-report.md
files:
  - crates/assay-types/src/lib.rs
---

## Problem

Naive serde serialization includes empty strings (`stdout: ""`), None values, and empty vectors in JSON output, adding 10-30% token overhead on MCP responses.

## Solution

Apply `#[serde(skip_serializing_if = "...")]` annotations on all `Option`, `String`, and `Vec` fields across domain types. This is mechanical, zero-risk, and should be a convention from Phase 3 onward.

Examples:
- `#[serde(skip_serializing_if = "Option::is_none")]`
- `#[serde(skip_serializing_if = "String::is_empty")]`
- `#[serde(skip_serializing_if = "Vec::is_empty")]`
