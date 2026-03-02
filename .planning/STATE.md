# State

## Current Position

Phase: 5 of 10 — Config and Initialization
Plan: 01 of 03
Status: In progress
Last activity: 2026-03-02 — Completed 05-01-PLAN.md (config type redesign and error variants)

Progress: [██░░░░░░░░] 23% (10/43 requirements)

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 23% |

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Workspace Prerequisites | Complete |
| 2 | MCP Spike | Complete (GO) |
| 3 | Error Types and Domain Model | Complete |
| 4 | Schema Generation | Complete |
| 5 | Config and Initialization | In Progress (Plan 1/3 complete) |
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
- AssayError::Io carries structured fields (PathBuf, String), no #[from] — context required at call sites
- #[non_exhaustive] on AssayError — new variants are non-breaking additions
- Result<T> alias for std::result::Result<T, AssayError> re-exported from assay-core root
- inventory::iter returns IntoIterator, not Iterator — all_entries() calls .into_iter()
- Rust cargo examples CAN access dev-dependencies — serde_json stays as dev-dep
- schemas-check NOT added to `just ready` to avoid circular dependency during development
- All 8 public types get individual schema files (not just top-level Config)
- Schema $id uses https://assay.dev/schemas/{name}.schema.json (aspirational URL convention)
- Generated schemas committed to git for IDE/consumer access without building
- Convention: every JsonSchema-derived type MUST have inventory::submit! immediately after definition
- ConfigError in config/mod.rs, not error.rs — config-specific validation output stays with config concerns
- toml dep added to assay-core for config loading (Plan 02)
- Existing Workflow/Gate types left untouched — only Config replaced; placeholders revisited later

### Blockers

None.

### Next Actions

1. Execute 05-02-PLAN.md — Config loading and validation via TDD (from_str, load, validate)
2. Execute 05-03-PLAN.md — Init logic in assay-core and CLI init subcommand

### Session Continuity

Last session: 2026-03-02
Stopped at: Completed 05-01-PLAN.md
Resume file: None
