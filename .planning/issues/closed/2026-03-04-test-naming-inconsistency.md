---
created: 2026-03-04T10:00
title: Standardize test naming conventions across crates
area: assay-core,assay-mcp
severity: suggestion
files:
  - crates/assay-mcp/src/server.rs
  - crates/assay-core/src/gate/mod.rs
---

## Problem

Test functions use inconsistent naming: some use `test_` prefix (MCP tests), others use no prefix (gate tests). This makes it harder to identify tests at a glance and suggests inconsistent conventions across the codebase.

## Solution

Establish and document a project-wide test naming convention (e.g., all tests use `#[test] fn test_*` or none do), then apply uniformly across all crates.


## Resolution

Closed as acknowledged in Phase 19 Plan 02 (2026-03-06). Test naming/structure suggestions are low-priority style preferences. Current naming is functional and consistent within each crate.
