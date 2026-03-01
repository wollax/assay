# State

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-02-28 — Milestone v0.1.0 started

## Accumulated Context

### Decisions

- assay-types = pub DTOs, zero logic; assay-core = free functions, all behavior
- CLI/MCP = thin wrappers delegating to core
- Config (Gate) ≠ State (GateResult) — never mix them
- Add error variants when consumed, not speculatively
- Criteria live on spec with optional `cmd` field (forward-compatible with `prompt` for agent track)
- MCP spike days 1-2 as GO/NO-GO gate
- M1 = foundation/proof of concept; M2 = launch/external demo

### Blockers

None.
