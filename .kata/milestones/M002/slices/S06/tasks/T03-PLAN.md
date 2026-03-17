---
estimated_steps: 5
estimated_files: 3
---

# T03: End-to-end integration tests with mock runners and real git repos

**Slice:** S06 — MCP Tools & End-to-End Integration
**Milestone:** M002

## Description

Create integration tests that exercise the full orchestrated path: DAG validation → parallel execution → scope-enforced harness config → sequential merge → status reporting. Uses mock session runners (not real agents) with real git repos (tempfile + git init). This is the capstone verification proving all M002 components compose correctly. Proves R020 and R021 at the integration level.

## Steps

1. **Create `crates/assay-core/tests/orchestrate_integration.rs`** with test infrastructure:
   - Helper to create temp git repo with initial commit, .assay dir, config.toml, and spec files
   - Mock session runner closure that creates a branch, writes files, and commits — simulating what a real agent would produce
   - Helper to write a multi-session manifest.toml to the temp dir

2. **Test: 3-session manifest with mixed dependencies succeeds end-to-end**:
   - Manifest: session A (no deps), session B (depends_on: [A]), session C (no deps)
   - Mock runner creates unique files per session (a.txt, b.txt, c.txt) on worktree branches
   - Call `run_orchestrated()` → verify A and C ran in parallel (or at least both completed), B ran after A
   - Call `extract_completed_sessions()` → `merge_completed_sessions()` on base branch
   - Assert: all three branches merged into base, all files present, MergeReport shows 3 merged / 0 skipped / 0 conflicted
   - Assert: state.json exists under `.assay/orchestrator/<run_id>/` with correct phases

3. **Test: Failure propagation — A fails, B skipped, C succeeds**:
   - Same manifest structure, but mock runner returns Err for session A
   - Call `run_orchestrated()` → verify B is Skipped (upstream 'A' failed), C is Completed
   - Merge only C's branch succeeds
   - Assert: OrchestratorResult has correct outcome types per session

4. **Test: `orchestrate_status` reads persisted state**:
   - In assay-mcp tests: create a temp project, write a state.json file to `.assay/orchestrator/<run_id>/state.json`
   - Call `orchestrate_status` handler with the run_id
   - Assert: response contains correct phase, session states, and run_id
   - Test missing run_id returns domain error

5. **Run `just ready`** — verify all checks pass (fmt, lint, test, deny) with all new code integrated

## Must-Haves

- [ ] Integration test with 3+ session manifest proving DAG → execute → merge path
- [ ] Integration test proving failure propagation (fail → skip dependents → continue independent)
- [ ] Integration test proving status file persistence and readability
- [ ] MCP handler test for `orchestrate_status` with pre-written state file
- [ ] All tests use real git repos (tempfile), not mocked git
- [ ] `just ready` passes

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate_integration` — 2+ integration tests pass
- `cargo test -p assay-mcp --features orchestrate -- orchestrate_status` — status handler test passes
- `just ready` — all checks pass

## Observability Impact

- Signals added/changed: None — tests verify existing observability surfaces
- How a future agent inspects this: Run the integration tests to verify orchestration pipeline health; test names indicate which phase is failing
- Failure state exposed: Test assertions check MergeReport contents, OrchestratorResult outcome types, and state.json structure

## Inputs

- T01 output — `orchestrate_status` handler in server.rs for MCP handler test
- T02 output — CLI routing logic confirmed working; integration tests exercise the same core functions
- `crates/assay-core/src/orchestrate/executor.rs` — `run_orchestrated()` as the execution engine
- `crates/assay-core/src/orchestrate/merge_runner.rs` — `merge_completed_sessions()`, `extract_completed_sessions()`
- `crates/assay-core/tests/` — existing test patterns in assay-core
- `crates/assay-mcp/tests/mcp_handlers.rs` — existing handler test patterns

## Expected Output

- `crates/assay-core/tests/orchestrate_integration.rs` — new file with 2+ integration tests using real git repos
- `crates/assay-mcp/tests/mcp_handlers.rs` — 1-2 new tests for orchestrate_status handler
- `just ready` green
