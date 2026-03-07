---
created: 2026-03-04T10:00
title: Log/warn on silent spec.toml parse errors in spec_get and spec show
area: assay-core
severity: important
files:
  - crates/assay-mcp/src/server.rs:202
  - crates/assay-cli/src/main.rs:291-296
---

## Problem

Parse errors in `spec_get()` and `spec show` are silently swallowed, making it difficult to debug configuration issues. Users see empty results with no indication that the spec failed to parse.

## Solution

Log or warn on parse errors before returning empty/default results. Consider whether errors should be propagated or gracefully degraded with user-visible warnings.
