# State

## Current Position

Phase: 1 of 10 — Workspace Prerequisites
Plan: 01-01 complete, 1 of 1 plans in phase
Status: In progress
Last activity: 2026-03-01 — Completed 01-01-PLAN.md (schemars upgrade + assay-mcp scaffold)

Progress: [█░░░░░░░░░] ~10%

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | ~2% |

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Workspace Prerequisites | In Progress (01-01 complete) |
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
- schemars uses caret range `"1"` (not exact pin) — matches rmcp's own declaration, picks up semver patches
- deny.toml required no changes for rmcp transitive deps — all licenses already in allow-list

### Blockers

None.

### Next Actions

1. Continue with remaining Phase 1 plans (if any exist)
2. Begin Phase 2 (MCP Spike) — rmcp and assay-mcp scaffold are ready

### Session Continuity

Last session: 2026-03-01T04:10:23Z
Stopped at: Completed 01-01-PLAN.md
Resume file: None
