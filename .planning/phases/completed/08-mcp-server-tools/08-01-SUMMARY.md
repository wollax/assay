---
phase: 08-mcp-server-tools
plan: 01
subsystem: mcp-server
tags: [rmcp, mcp, server, tools, spec, gate]
dependency-graph:
  requires: [07-gate-evaluation, 06-spec-files, 05-config-and-init, 02-mcp-spike]
  provides: [AssayServer with spec_get/spec_list/gate_run tools, bounded response formatting, domain error handling]
  affects: [08-02 integration tests, 09-cli-surface, 10-claude-code-plugin]
tech-stack:
  added: []
  patterns: [tool_router/tool_handler macros, Parameters<T> for schema generation, spawn_blocking for sync bridge, CallToolResult::error for domain errors, per-call config resolution]
key-files:
  created:
    - crates/assay-mcp/src/server.rs
  modified:
    - crates/assay-mcp/src/lib.rs
    - crates/assay-mcp/Cargo.toml
    - Cargo.lock
  deleted:
    - crates/assay-mcp/src/spike.rs
decisions:
  - "assay-types added as direct dependency to assay-mcp for type naming in function signatures"
  - "chrono added as dev-dependency to assay-mcp for GateResult construction in tests"
  - "first_nonempty_line helper extracts failure reason from stderr for summary mode"
  - "failed criteria with empty stderr get 'unknown' as reason"
metrics:
  duration: ~8 minutes
  completed: 2026-03-02
---

# Phase 8 Plan 01: AssayServer with MCP Tools Summary

Replace the Phase 2 spike server with a real MCP server exposing three tools (spec_get, spec_list, gate_run) with bounded responses, self-documenting descriptions, and proper domain error handling via CallToolResult::error.

## What Was Done

### Task 1: Delete spike module and create AssayServer with tools and helpers

**Spike removal:**
- Deleted `crates/assay-mcp/src/spike.rs` (Phase 2 throwaway code)
- No SpikeServer references remain anywhere in the source codebase

**Server implementation (server.rs, 610 lines):**

1. **Parameter structs** with `JsonSchema + Deserialize` derives:
   - `SpecGetParams` with `name` field (schemars description includes example)
   - `GateRunParams` with `name` and `include_evidence` (bool, default false)
   - `spec_list` takes no parameters (rmcp auto-generates empty object schema)

2. **Response structs** with `Serialize` and `skip_serializing_if`:
   - `SpecListEntry` — name, description (skipped when empty), criteria_count
   - `GateRunResponse` — spec_name, passed/failed/skipped counts, total_duration_ms, criteria
   - `CriterionSummary` — name, status, optional exit_code/duration_ms/reason/stdout/stderr

3. **AssayServer struct** with `ToolRouter<Self>`, Clone derive, Default impl

4. **Three MCP tools** in `#[tool_router] impl AssayServer`:
   - `spec_list` — scans specs directory, returns array of entries
   - `spec_get` — loads a spec by name, returns full JSON
   - `gate_run` — evaluates all criteria via `spawn_blocking`, returns bounded summary (or full evidence when `include_evidence=true`)

5. **ServerHandler impl** with `get_info()` returning agent-oriented instructions

6. **Seven helper functions:**
   - `resolve_cwd()` — McpError on failure
   - `load_config()` — returns `Result<Config, CallToolResult>`
   - `load_spec()` — returns `Result<Spec, CallToolResult>`
   - `resolve_working_dir()` — matches CLI behavior exactly
   - `domain_error()` — converts AssayError to CallToolResult::error (isError: true)
   - `format_gate_response()` — maps GateRunSummary to bounded GateRunResponse
   - `first_nonempty_line()` — extracts failure reason from stderr

7. **`serve()` function** — creates server, starts stdio transport

8. **Six unit tests:**
   - `test_format_gate_response_summary_mode` — verifies counts, per-criterion status, no stdout/stderr in summary
   - `test_format_gate_response_evidence_mode` — verifies stdout/stderr present when include_evidence=true
   - `test_domain_error_produces_error_result` — verifies isError: true and error text
   - `test_first_nonempty_line` — verifies edge cases (empty, blank lines, multi-line)
   - `test_format_gate_response_failed_with_empty_stderr` — verifies "unknown" reason fallback

**lib.rs update:**
- Replaced `mod spike;` with `mod server;`
- `serve()` delegates to `server::serve()`
- Module doc comment updated to list three real tools

**Cargo.toml changes:**
- Added `assay-types.workspace = true` to dependencies
- Added `chrono.workspace = true` to dev-dependencies

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Added assay-types as direct dep | Function signatures need to name Config and Spec types; transitive access through assay-core doesn't allow type naming |
| Added chrono as dev-dep | Unit tests construct GateResult values which require DateTime<Utc> for timestamp field |
| first_nonempty_line for failure reason | Extracts actionable error context from stderr without including full output in summary mode |
| "unknown" for empty stderr failures | Silent command failures still need a reason field; "unknown" is honest and machine-parseable |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added assay-types as direct dependency**
- **Found during:** Task 1, initial build
- **Issue:** Plan stated "assay-types is accessed transitively through assay-core" but function signatures like `fn load_config(cwd: &Path) -> Result<Config, CallToolResult>` need to name the `Config` type, which requires a direct dependency
- **Fix:** Added `assay-types.workspace = true` to assay-mcp/Cargo.toml dependencies
- **Files modified:** crates/assay-mcp/Cargo.toml
- **Commit:** 93cba26

**2. [Rule 3 - Blocking] Added chrono as dev-dependency**
- **Found during:** Task 1, test compilation
- **Issue:** Unit tests construct `GateResult` values which require `chrono::Utc` for the `timestamp` field
- **Fix:** Added `chrono.workspace = true` to assay-mcp/Cargo.toml dev-dependencies
- **Files modified:** crates/assay-mcp/Cargo.toml
- **Commit:** 93cba26

## Verification Results

- `just ready` passes (fmt-check + lint + test + deny)
- `cargo doc -p assay-mcp --no-deps` succeeds
- No references to `spike` or `SpikeServer` in source code
- 99 tests pass workspace-wide (70 core + 5 mcp + 9 types + 15 schema)
- server.rs: 610 lines (min 250)
- lib.rs: 18 lines (min 8)

## Next Phase Readiness

Plan 08-02 (integration tests, CLI wiring, `just ready` gate) is unblocked. The server is complete and ready for:
1. Integration tests that start the full MCP server over stdio
2. CLI `mcp serve` subcommand wiring
3. End-to-end validation with Claude Code

No blockers or concerns.
