---
id: T01
parent: S06
milestone: M002
provides:
  - orchestrate_run MCP tool registered in router
  - orchestrate_status MCP tool registered in router
  - OrchestrateRunParams and OrchestrateStatusParams re-exported for integration tests
key_files:
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/src/lib.rs
key_decisions:
  - Session runner closure wraps plain function calls (generate_config/write_config/build_cli_args) inside a HarnessWriter Box, then delegates to run_session — satisfies D035 while reusing the full pipeline
  - Base branch detected via git rev-parse --abbrev-ref HEAD before orchestration, then checked out between execution and merge phases
  - Git checkout uses std::process::Command directly since assay-core's git_command is pub(crate)
patterns_established:
  - Orchestrate tool param/response types follow same Deserialize+JsonSchema pattern as existing 20 tools
  - Error path returns domain_error with structured messages for missing manifests, invalid run_ids, parse failures
observability_surfaces:
  - orchestrate_run returns structured JSON with run_id, per-session outcomes, merge report
  - orchestrate_status reads persisted OrchestratorStatus from .assay/orchestrator/<run_id>/state.json
  - isError: true with descriptive messages for missing manifests, invalid run_ids, checkout failures
duration: 20m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add orchestrate_run and orchestrate_status MCP tools

**Added orchestrate_run and orchestrate_status MCP tools to AssayServer, bringing tool count from 20 to 22.**

## What Happened

Added two new MCP tools following the established pattern:

1. **`orchestrate_run`**: Accepts `manifest_path`, optional `timeout_secs`, `failure_policy`, and `merge_strategy`. Validates multi-session content (sessions.len() > 1 or any depends_on), builds `OrchestratorConfig` and `PipelineConfig`, detects base branch via git, wraps the full orchestration + merge pipeline in `spawn_blocking` (D007). The session runner closure constructs a `HarnessWriter` from plain function calls (D035) and delegates to `run_session`. After orchestration, checks out the base branch and runs `extract_completed_sessions` → `merge_completed_sessions` with `default_conflict_handler`. Returns combined JSON with run_id, per-session outcomes, summary counts, and merge report.

2. **`orchestrate_status`**: Accepts `run_id`, reads `.assay/orchestrator/<run_id>/state.json`, deserializes `OrchestratorStatus`, returns as pretty-printed JSON. Domain errors for missing/invalid state files.

Added 11 unit tests covering param deserialization (full + minimal), schema generation, router registration, missing-manifest error, missing-run-id error, and successful state read.

Re-exported `OrchestrateRunParams` and `OrchestrateStatusParams` in `lib.rs` under `cfg(any(test, feature = "testing"))`.

## Verification

- `cargo test -p assay-mcp --lib` — 106 tests pass (was ~95 before, +11 new orchestrate tests)
- `cargo test -p assay-mcp` — all 106 unit + 27 integration tests pass
- `cargo clippy -p assay-mcp -- -D warnings` — clean
- Both tools appear in `list_all()` router output (verified by `orchestrate_run_tool_in_router` and `orchestrate_status_tool_in_router` tests)

### Slice-level verification (partial — T01 is first of 3 tasks):
- ✅ `cargo test -p assay-mcp` — new orchestrate_run/orchestrate_status handler tests pass
- ⬜ `cargo test -p assay-cli -- run` — CLI routing tests (T02)
- ⬜ `cargo test -p assay-core --features orchestrate -- integration` — end-to-end integration (T03)
- ⬜ `just ready` — deferred to T03

## Diagnostics

- Call `orchestrate_status` with a run_id to inspect persisted orchestrator state
- `orchestrate_run` response includes `merge_report` with per-session merge status and conflict details
- Failed runs return `isError: true` with descriptive messages including the failing stage/reason

## Deviations

- Task plan references `cargo test -p assay-mcp --features orchestrate` but assay-mcp has no `orchestrate` feature (it depends on assay-core with that feature enabled). Used `cargo test -p assay-mcp` instead.
- Used `std::process::Command` for git checkout instead of `assay_core::merge::git_command` which is `pub(crate)` and not accessible from assay-mcp.
- Session runner delegates to `run_session` (which handles the full pipeline including worktree creation, agent launch, gate evaluation, merge check) rather than reimplementing the pipeline. This satisfies D035 because the HarnessWriter closure is constructed from plain function calls, not passed in as a dyn.

## Known Issues

- The `orchestrate_run` handler's session runner constructs a full HarnessWriter and calls `run_session` which includes agent launch — in integration tests this would require a real or mock agent binary. The T03 integration tests will need mock session runners.

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` — Added OrchestrateRunParams, OrchestrateStatusParams, response types, orchestrate_run handler, orchestrate_status handler, and 11 unit tests
- `crates/assay-mcp/src/lib.rs` — Re-exported OrchestrateRunParams and OrchestrateStatusParams under testing cfg
