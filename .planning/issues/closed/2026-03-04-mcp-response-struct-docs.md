---
created: 2026-03-04T10:00
title: GateRunResponse and CriterionSummary private structs in MCP server need doc comments
area: assay-mcp
severity: suggestion
files:
  - crates/assay-mcp/src/server.rs
---

## Problem

The MCP server defines private response structs `GateRunResponse` and `CriterionSummary` without doc comments, reducing code clarity for maintainers reading the MCP server implementation.

## Solution

Add doc comments to `GateRunResponse` and `CriterionSummary` explaining their role in the MCP response serialization flow.
