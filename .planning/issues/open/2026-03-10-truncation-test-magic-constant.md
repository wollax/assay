---
created: 2026-03-10T13:50
title: Add comment for 524_288 magic constant in truncation tests
area: testing
provenance: local
files:
  - crates/assay-mcp/src/server.rs:3197
---

## Problem

The truncation visibility tests use `524_288` (512 KiB) as a test value for `original_bytes` without any comment explaining its significance.

## Solution

Add inline comment `// 512 KiB` next to the constant, or extract to a named test constant.
