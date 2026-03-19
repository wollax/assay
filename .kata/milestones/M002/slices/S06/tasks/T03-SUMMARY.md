---
id: T03
parent: S06
milestone: M002
provides:
  - 3 end-to-end integration tests proving DAG → execute → merge → status path with real git repos
  - 2 MCP handler tests for orchestrate_status with pre-written state files
key_files:
  - crates/assay-core/tests/orchestrate_integration.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
key_decisions:
  - Mock runners must git-add only specific files (not `git add .`) to avoid staging .assay/orchestrator/ state files which get deleted on branch checkout
  - .assay/.gitignore with `orchestrator/` entry needed in test repos so merge runner sees clean worktree
patterns_established:
  - Integration test pattern: tempdir + git init + .assay setup + .gitignore for orchestrator dir + mock runner that creates real branches/commits
observability_surfaces:
  - Tests verify state.json persistence, phase transitions, and per-session status fields — serves as regression suite for orchestration observability
duration: 25m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: End-to-end integration tests with mock runners and real git repos

**Added 3 integration tests with real git repos proving full DAG → execute → merge → status path, plus 2 MCP handler tests for orchestrate_status.**

## What Happened

Created `crates/assay-core/tests/orchestrate_integration.rs` with test infrastructure: helper to create temp git repos with initial commit, `.assay` directory, and `.gitignore` for orchestrator state; mock session runner that creates real git branches, writes unique files, and commits; helper to build manifests from tuples.

Three integration tests exercise the full orchestration pipeline:
1. **3-session DAG execute+merge** — sessions A (no deps), B (depends on A), C (no deps) all succeed. Verifies all branches merge into base, all files present, MergeReport shows 3 merged / 0 conflicts, and state.json persists with `Completed` phase.
2. **Failure propagation** — A fails, B skipped (upstream failed), C succeeds. Verifies correct outcome types, only C merges, state.json shows `PartialFailure` phase with correct per-session states.
3. **Status persistence round-trip** — 2-session run, reads state.json back, verifies all fields (run_id, phase, failure_policy, per-session timing/state), and confirms JSON round-trip serialization.

Added 2 MCP handler tests in `mcp_handlers.rs`: one that writes a realistic multi-session state.json and calls `orchestrate_status` to verify the response contains correct phase, sessions, and run_id; one that tests missing run_id returns a domain error.

Key discovery: mock runners that use `git add .` inadvertently stage `.assay/orchestrator/` state files on session branches. When `git checkout main` runs, git deletes those files since they're not tracked on main — causing the final `persist_state` to silently fail (writes to a deleted directory). Fixed by having mock runners `git add <specific-file>` and adding `.assay/.gitignore` with `orchestrator/` to keep the merge runner's clean-worktree check happy.

## Verification

- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — 3 tests pass ✅
- `cargo test -p assay-mcp -- orchestrate_status` — 7 tests pass (5 unit + 2 integration) ✅
- `just ready` — all checks pass (fmt, lint, test, deny) ✅

Slice-level verification:
- `cargo test -p assay-mcp --features orchestrate` — ✅ (orchestrate_run/orchestrate_status handler tests pass)
- `cargo test -p assay-cli -- run` — ✅ (multi-session CLI routing tests pass)
- `cargo test -p assay-core --features orchestrate -- integration` — ✅ (end-to-end integration tests pass)
- `just ready` — ✅ all checks pass

## Diagnostics

- Run `cargo test -p assay-core --features orchestrate --test orchestrate_integration` to verify orchestration pipeline health
- Test names indicate which phase is failing: `three_session_dag_execute_merge_end_to_end` (happy path), `failure_propagation_a_fails_b_skipped_c_succeeds` (error handling), `status_persistence_round_trip` (state I/O)
- Tests assert on MergeReport contents, SessionOutcome types, OrchestratorStatus fields, and actual file presence on disk

## Deviations

- Added `.assay/.gitignore` with `orchestrator/` in test setup — not in the original plan but required to prevent merge runner's clean-worktree check from rejecting the untracked orchestrator state directory. Real projects will need this too.

## Known Issues

- Real projects using orchestration will also need `.assay/orchestrator/` in their `.gitignore` to avoid the same clean-worktree issue during merge phase. This should be handled by the project scaffolding/init code.

## Files Created/Modified

- `crates/assay-core/tests/orchestrate_integration.rs` — new file with 3 integration tests using real git repos
- `crates/assay-mcp/tests/mcp_handlers.rs` — added 2 tests for orchestrate_status handler
