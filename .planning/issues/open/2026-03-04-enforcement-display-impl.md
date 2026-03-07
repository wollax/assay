---
created: 2026-03-04T10:00
title: Enforcement should implement Display instead of ad-hoc match in MCP server
area: assay-types
severity: suggestion
files:
  - crates/assay-types/src/enforcement.rs
  - crates/assay-mcp/src/server.rs
---

## Problem

The MCP server contains ad-hoc match expressions to convert `Enforcement` enum to string format, rather than implementing a standard `Display` trait on the enum itself. This scatters formatting logic and reduces code reusability.

## Solution

Implement `Display` trait for `Enforcement` in `assay-types` and use it throughout the codebase for consistent string conversion.
