> **Closed:** 2026-03-15 — Won't fix. Superseded by v0.4.0 architecture (phases 35-44).


---
created: 2026-03-01T05:30
title: Design separate wire (MCP) vs display (CLI) format types
area: assay-types
phase: 3
provenance: brainstorm:2026-02-28T23-16-brainstorm/architecture-report.md
files:
  - crates/assay-types/src/lib.rs
---

## Problem

A single `GateResult` type used for both MCP responses and CLI display leads to either verbose JSON (bad for agents) or terse output (bad for humans). Designing for compact output from the start avoids retrofitting compression.

## Solution

Design wire format types with compact-but-readable field names for MCP consumption, separate from CLI display rendering. Both derive from the same underlying `GateResult`.

- **Wire format (MCP):** Use `#[serde(rename)]` for compact field names (`pass`, `exit`, `dur`). No prose, no ANSI. Structured JSON for agents.
- **Display format (CLI/TUI):** Separate rendering functions for human-readable output.

This is a type design decision in Phase 3, with MCP-specific response types landing in Phase 8.