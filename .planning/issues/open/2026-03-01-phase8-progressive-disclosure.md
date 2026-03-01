---
created: 2026-03-01T05:30
title: Implement progressive gate disclosure as two-tool MCP pattern
area: assay-mcp
phase: 8
provenance: brainstorm:2026-02-28T23-16-brainstorm/ai-compression-report.md
files:
  - crates/assay-mcp/src/lib.rs
---

## Problem

Returning full stdout/stderr for all criteria in a single `gate_run` response produces unbounded, token-expensive responses. A gate run with 8 criteria at 50KB each is 400KB — far too much for an agent's context window.

## Solution

Split gate evaluation into a two-tool MCP pattern:

**`gate_run`** — Returns bounded-size structured summary:
- Per-criterion pass/fail, exit code, and deterministic `reason` (extracted failure lines)
- Passing criteria: `reason: null`
- Failing criteria: last meaningful non-noise lines from stderr/stdout
- Overall summary ("4/6 criteria passed")

**`gate_evidence`** — Returns full raw stdout/stderr for a specific criterion + run:
- Input: spec name, criterion name, run_id
- Output: full stdout, stderr, exit_code

Evidence lifecycle:
- Stored in-memory within MCP server process
- All runs retained (enables cross-run comparison)
- Memory bounded: ~10 runs × 8 criteria × 50KB = 4MB max
- Dies with server process (stdio = one client per process)

**Architectural principle:** Every Assay MCP tool response must be bounded in size. Tools producing variable-size output use summary+drill-down.

Use wire format types (from Phase 3) for compact JSON responses. Apply serde hygiene.
