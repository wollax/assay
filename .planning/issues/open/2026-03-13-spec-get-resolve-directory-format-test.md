---
created: 2026-03-13T10:45
title: Add spec_get resolve=true test for directory-format specs
area: mcp
provenance: local
files:
  - crates/assay-mcp/src/server.rs:628-643
---

## Problem

All spec_get resolve integration tests use legacy-format specs (`[[criteria]]` in a single TOML). The `resolved_block` insertion code path is shared between `SpecEntry::Legacy` and `SpecEntry::Directory` branches, but directory-format specs are not covered by tests. A bug in the Directory branch that drops the `resolved` key or serializes it incorrectly would go undetected.

## Solution

Add a test that creates a directory-format spec (with `gates.toml` under `.assay/specs/<name>/`) and calls `spec_get` with `resolve: true`, verifying the resolved block appears correctly.
