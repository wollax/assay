---
created: 2026-03-02T14:30
title: spec_list silently discards scan errors
area: mcp
provenance: github:wollax/assay#38
severity: critical
files:
  - crates/assay-mcp/src/server.rs:127-140
---

## Problem

`spec_list` tool silently discards `scan_result.errors` from `assay_core::spec::scan`. When individual spec files fail to parse, agents see only valid specs with no indication anything is wrong. An agent calling `spec_list` in a project where 3 of 5 specs have bad TOML will see only 2 entries and no warning. The CLI handles this correctly by iterating `result.errors` and emitting warnings.

If all specs are broken, the tool returns an empty array with `isError: false` — a fully silent failure.

## Solution

Surface errors in the response. Options:
1. Append warnings as additional `Content::text` items in the `CallToolResult`
2. Add a `warnings` or `errors` field to the response JSON
3. Return `CallToolResult::error` when errors are present alongside an empty spec list

## Resolution

Resolved during Phase 17-01. `SpecListResponse` includes `errors` field.
