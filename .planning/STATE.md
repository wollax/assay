# State

## Current Position

Phase: 2 of 10 — MCP Spike
Status: Phase 2 Plan 01 Complete (GO decision confirmed)
Last activity: 2026-03-01 — MCP spike validated end-to-end

Progress: [██░░░░░░░░] 10% (6/43 requirements)

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 10% |

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Workspace Prerequisites | Complete |
| 2 | MCP Spike | In Progress (Plan 01 complete — GO) |
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
- schemars uses caret range `"1"` (not exact pin) — matches rmcp's own declaration, picks up semver patches
- deny.toml required no changes for rmcp transitive deps — all licenses already in allow-list
- **MCP Spike: GO** — rmcp 0.17 + stdio + Claude Code integration path confirmed
- rmcp's `#[tool_router]` / `#[tool_handler]` macro pattern works cleanly for tool registration
- `tracing-subscriber` stderr-only writer keeps stdout clean for JSON-RPC (no byte leakage)
- `Implementation::from_build_env()` populates server info from Cargo.toml automatically
- Spike code remains as working reference until Phase 8 replaces with real tools

### Blockers

None.

### Next Actions

1. Begin Phase 3 (Error Types and Domain Model) — unblocked
2. Phases 3-10 proceed on confirmed rmcp architecture

### Session Continuity

Last session: 2026-03-01
Stopped at: Phase 2 Plan 01 complete (GO decision)
Resume file: None
