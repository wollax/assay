# S06: MCP Tools & End-to-End Integration

**Goal:** Wire all M002 components into a working end-to-end system: MCP tools expose orchestration, CLI routes multi-session manifests to the orchestrator with post-execution merge, and integration tests prove the full path.
**Demo:** A 3+ session manifest with mixed dependencies runs through `run_orchestrated()` → scope-enforced harness config → sequential merge → status reporting. `orchestrate_run` MCP tool launches orchestration. `orchestrate_status` reads live state. Single-session manifests still use the existing `run_manifest()` path. `just ready` passes.

## Must-Haves

- `orchestrate_run` MCP tool: accepts manifest path, detects multi-session, calls `run_orchestrated()` + `merge_completed_sessions()`, returns combined response with execution outcomes + merge report
- `orchestrate_status` MCP tool: accepts run_id, reads `.assay/orchestrator/<run_id>/state.json`, returns `OrchestratorStatus`
- CLI routing: `assay run` detects multi-session manifests (sessions.len() > 1 or any depends_on) and routes to orchestrator + merge, single-session unchanged
- `--failure-policy` and `--merge-strategy` CLI flags on `assay run`
- Integration tests proving: DAG validation, parallel execution with failure propagation, sequential merge in topological order, status file persistence
- `just ready` passes with all new code

## Proof Level

- This slice proves: final-assembly (all M002 components wired end-to-end)
- Real runtime required: no (mock session runners, real git repos)
- Human/UAT required: yes (real agent invocation is manual UAT)

## Verification

- `cargo test -p assay-mcp --features orchestrate` — new orchestrate_run/orchestrate_status handler tests
- `cargo test -p assay-cli -- run` — multi-session CLI routing tests
- `cargo test -p assay-core --features orchestrate -- integration` — end-to-end integration tests with real git repos
- `just ready` — all checks pass (fmt, lint, test, deny)

## Observability / Diagnostics

- Runtime signals: `OrchestratorStatus` persisted to `.assay/orchestrator/<run_id>/state.json` after each session; `MergeReport` returned with per-session merge status
- Inspection surfaces: `orchestrate_status` MCP tool reads persisted state; CLI `--json` output includes orchestration and merge phases
- Failure visibility: `PipelineError` with stage/message/recovery; `MergeReport` with per-session conflict details; `OrchestratorPhase::PartialFailure` when sessions fail
- Redaction constraints: none (no secrets in orchestration state)

## Integration Closure

- Upstream surfaces consumed: `run_orchestrated()` (S02), `merge_completed_sessions()` / `extract_completed_sessions()` / `default_conflict_handler()` (S03), `inject_scope_layer()` pattern (S05), DAG validation (S01), all three adapter generate/write functions (S04)
- New wiring introduced in this slice: `orchestrate_run` and `orchestrate_status` MCP tools; CLI multi-session routing with orchestrator + merge; scope prompt injection during orchestrated runs
- What remains before the milestone is truly usable end-to-end: Manual UAT with real agents (not automatable). Everything else is delivered.

## Tasks

- [x] **T01: Add orchestrate_run and orchestrate_status MCP tools** `est:45m`
  - Why: R021 requires additive MCP tools for orchestration. These are the programmatic entry points for multi-agent orchestration.
  - Files: `crates/assay-mcp/src/server.rs`, `crates/assay-mcp/src/lib.rs`
  - Do: Add param/response structs for both tools. `orchestrate_run` loads manifest, detects multi-session, builds `OrchestratorConfig` + `PipelineConfig`, wraps `run_orchestrated()` in `spawn_blocking`, then calls `merge_completed_sessions()` with `default_conflict_handler()`, returns combined JSON response. `orchestrate_status` reads state.json by run_id. Harness writer constructed inside session runner closure using plain function calls (D035). Add to `#[tool_router]`. Re-export param types under `cfg(any(test, feature = "testing"))` in lib.rs.
  - Verify: `cargo test -p assay-mcp --features orchestrate` — param deserialization, schema generation, router registration, and missing-manifest error handling tests pass
  - Done when: Both tools registered in router, schema tests pass, handler tests for error paths pass

- [x] **T02: Wire CLI multi-session routing with orchestrator and merge** `est:40m`
  - Why: R020 requires `assay run` to detect multi-session manifests and route to the orchestrator. This is the user-facing entry point.
  - Files: `crates/assay-cli/src/commands/run.rs`
  - Do: Add `--failure-policy` (skip-dependents|abort) and `--merge-strategy` (completion-time|file-overlap) flags. Detect multi-session (sessions.len() > 1 or any depends_on). Multi-session path: build `OrchestratorConfig`, construct Sync session runner closure using plain harness function calls (D035), call `run_orchestrated()`, checkout base branch, call `merge_completed_sessions()` with `default_conflict_handler()`, format orchestration + merge results. Single-session path: unchanged `run_manifest()`. Add JSON response types for orchestrated results.
  - Verify: `cargo test -p assay-cli -- run` — existing tests still pass, new tests for multi-session detection logic and flag parsing
  - Done when: `assay run` correctly routes single vs multi-session manifests, new flags parse correctly, existing CLI tests unbroken

- [x] **T03: End-to-end integration tests with mock runners and real git repos** `est:50m`
  - Why: The milestone requires a 3+ session manifest exercising DAG → parallel execution → merge → status. This proves the full assembly works, not just individual components.
  - Files: `crates/assay-core/tests/orchestrate_integration.rs`, `crates/assay-mcp/tests/mcp_handlers.rs`
  - Do: Create integration test in assay-core with real git repos (using tempfile + git init): (1) 3-session manifest with A→B dependency, C independent — mock runner creates commits on worktree branches — verify parallel execution, correct ordering, merge into base branch. (2) Failure propagation — A fails, B skipped, C succeeds. (3) Status file persistence — verify state.json written with correct phases. Add MCP handler test calling `orchestrate_status` with a pre-written state.json. Verify `just ready` passes.
  - Verify: `cargo test -p assay-core --features orchestrate -- orchestrate_integration` and `just ready`
  - Done when: 3+ integration tests pass proving full DAG→execute→merge→status path, `just ready` green

## Files Likely Touched

- `crates/assay-mcp/src/server.rs`
- `crates/assay-mcp/src/lib.rs`
- `crates/assay-cli/src/commands/run.rs`
- `crates/assay-core/tests/orchestrate_integration.rs`
- `crates/assay-mcp/tests/mcp_handlers.rs`
