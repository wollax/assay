---
created: 2026-03-13T10:45
title: Remove duplicate description on SpecGetParams.resolve field
area: mcp
provenance: local
files:
  - crates/assay-mcp/src/server.rs:51-56
---

## Problem

The `resolve` field on `SpecGetParams` has both a doc comment and a `#[schemars(description = ...)]` attribute with identical text. The duplication creates a maintenance burden — changes must be kept in sync.

## Solution

Remove the doc comment and keep only the `#[schemars]` attribute, consistent with the existing pattern for other param structs.
