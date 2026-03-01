# State

## Current Position

Phase: 1 of 10 — Workspace Prerequisites
Plan: v0.1.0 Proof of Concept (10 phases, 43 requirements)
Status: Ready to begin Phase 1
Last activity: 2026-02-28 — Roadmap created

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 0% |

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Workspace Prerequisites | Not Started |
| 2 | MCP Spike | Not Started |
| 3 | Error Types and Domain Model | Not Started |
| 4 | Schema Generation | Not Started |
| 5 | Config and Initialization | Not Started |
| 6 | Spec Files | Not Started |
| 7 | Gate Evaluation | Not Started |
| 8 | MCP Server Tools | Not Started |
| 9 | CLI Surface Completion | Not Started |
| 10 | Claude Code Plugin | Not Started |

## Accumulated Context

### Decisions

- assay-types = pub DTOs, zero logic; assay-core = free functions, all behavior
- CLI/MCP = thin wrappers delegating to core
- Config (Gate) != State (GateResult) — never mix them
- Add error variants when consumed, not speculatively
- Criteria live on spec with optional `cmd` field (forward-compatible with `prompt` for agent track)
- MCP spike days 1-2 as GO/NO-GO gate
- M1 = foundation/proof of concept; M2 = launch/external demo
- schemars 0.8 -> 1.x is mandatory prerequisite (rmcp requires it)
- assay-mcp is a library crate, not a binary — single `assay` binary for all surfaces
- `Command::output()` for gate execution (not spawn+wait) to avoid pipe buffer deadlock
- `spawn_blocking` for sync gate evaluation in async MCP handlers
- `#[serde(tag = "kind")]` internal tagging on GateKind for TOML compatibility

### Blockers

None.

### Next Actions

1. Upgrade schemars 0.8 -> 1.x in workspace Cargo.toml
2. Add workspace deps: rmcp, toml, tracing, tracing-subscriber
3. Create `crates/assay-mcp/` crate scaffold
4. Run `just ready` to confirm clean build
