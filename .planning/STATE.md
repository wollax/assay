# State

## Current Position

Phase: 3 of 10 — Error Types and Domain Model
Plan: 01 of 02
Status: In progress
Last activity: 2026-03-01 — Completed 03-01-PLAN.md (domain types)

Progress: [██░░░░░░░░] 9% (4/43 requirements)

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 9% |

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Workspace Prerequisites | Complete |
| 2 | MCP Spike | Complete (GO) |
| 3 | Error Types and Domain Model | In Progress (Plan 01 complete) |
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
- GateResult does not derive PartialEq — DateTime equality is semantically questionable
- serde_json moved to dev-dependencies in assay-types (source files don't use it)
- schemars chrono04 feature enabled at workspace level for DateTime<Utc> JsonSchema support

### Blockers

None.

### Next Actions

1. Execute Phase 3 Plan 02 (error types in assay-core) — unblocked
2. Phases 4-10 proceed on confirmed architecture

### Session Continuity

Last session: 2026-03-01
Stopped at: Phase 3, Plan 01 complete
Resume file: .planning/phases/active/03-error-types-and-domain-model/03-01-SUMMARY.md
