---
created: 2026-03-13T10:45
title: Avoid clone on resolved_block insertion in spec_get
area: mcp
provenance: local
files:
  - crates/assay-mcp/src/server.rs:623-641
---

## Problem

The `resolved_block` variable is cloned twice via `resolved.clone()` when inserting into Legacy and Directory response branches. Since only one branch is taken, a consuming move would avoid the clone.

## Solution

Restructure to move `resolved_block` into the taken branch, e.g., by building the response with the resolved field inline or using `if let Some(resolved) = resolved_block` (consuming) after the match.
