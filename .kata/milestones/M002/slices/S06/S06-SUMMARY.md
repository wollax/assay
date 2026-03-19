---
id: S06
parent: M002
milestone: M002
provides:
  - orchestrate_run MCP tool (launches multi-session orchestration + merge)
  - orchestrate_status MCP tool (reads live session state from disk)
  - CLI multi-session routing (assay run detects single vs multi-session)
  - --failure-policy and --merge-strategy CLI flags
  - 3 end-to-end integration tests with real git repos proving DAG → execute → merge → status
  - 2 MCP handler integration tests for orchestrate_status
requires:
  - slice: S01
    provides: DependencyGraph, DAG validation, topological_groups
  - slice: S02
    provides: run_orchestrated(), OrchestratorStatus, SessionOutcome, state persistence
  - slice: S03
    provides: merge_completed_sessions(), extract_completed_sessions(), default_conflict_handler(), MergeReport
  - slice: S04
    provides: Codex and OpenCode adapter generate/write functions
  - slice: S05
    provides: Harness CLI commands, scope enforcement, inject_scope_layer
affects: []
key_files:
  - crates/assay-mcp/src/server.rs
  - crates/assay-mcp/src/lib.rs
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-core/tests/orchestrate_integration.rs
  - crates/assay-mcp/tests/mcp_handlers.rs
key_decisions:
  - D039 — Multi-session detection heuristic (sessions.len() > 1 OR any depends_on)
  - D040 — Hardcode Claude Code adapter in orchestrated runs (per-session adapter selection deferred)
  - D041 — .assay/orchestrator/ must be gitignored to prevent state file interference with merge
patterns_established:
  - CLI orchestration path mirrors MCP tool pattern: session_runner closure → run_orchestrated → checkout base → extract_completed → merge_completed
  - Integration test pattern: tempdir + git init + .assay setup + .gitignore for orchestrator dir + mock runner with real branches/commits
observability_surfaces:
  - orchestrate_run returns structured JSON with run_id, per-session outcomes, merge report
  - orchestrate_status reads persisted OrchestratorStatus from .assay/orchestrator/<run_id>/state.json
  - CLI stderr shows phase markers (Phase 1/2/3), --json returns OrchestrationResponse
  - Exit codes: 0 = all succeed + clean merge, 1 = any error/skip, 2 = merge conflicts
drill_down_paths:
  - .kata/milestones/M002/slices/S06/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S06/tasks/T02-SUMMARY.md
  - .kata/milestones/M002/slices/S06/tasks/T03-SUMMARY.md
duration: 60m
verification_result: passed
completed_at: 2026-03-17
---

# S06: MCP Tools & End-to-End Integration

**Wired all M002 components end-to-end: MCP tools expose orchestration, CLI routes multi-session manifests through DAG execution + merge, integration tests prove the full path with real git repos.**

## What Happened

This capstone slice assembled all M002 components into a working end-to-end system across three tasks:

**T01 — MCP tools (20m):** Added `orchestrate_run` and `orchestrate_status` to AssayServer, bringing the tool count from 20 to 22. `orchestrate_run` accepts a manifest path, validates multi-session content, builds orchestrator + pipeline config, detects the base branch via git, wraps the full pipeline (orchestrated execution → base checkout → sequential merge) in `spawn_blocking`, and returns combined JSON with run_id, outcomes, and merge report. `orchestrate_status` reads persisted state from `.assay/orchestrator/<run_id>/state.json`. 11 unit tests cover param deserialization, schema generation, router registration, and error paths.

**T02 — CLI routing (15m):** Extended `assay run` with `--failure-policy` (skip-dependents|abort) and `--merge-strategy` (completion-time|file-overlap) flags. Added `needs_orchestration()` detection function that routes multi-session manifests to the orchestrator while leaving single-session unchanged. The orchestrated path mirrors the MCP tool: three-phase execution with stderr phase markers and structured JSON output via `--json`. 8 new tests cover detection logic, flag parsing, and response serialization.

**T03 — Integration tests (25m):** Created 3 end-to-end integration tests with real git repos (tempdir + git init + mock runners creating real branches/commits): (1) 3-session DAG with A→B dependency and independent C — all merge into base; (2) failure propagation — A fails, B skipped, C succeeds and merges alone; (3) status persistence round-trip verifying all OrchestratorStatus fields. Plus 2 MCP handler tests for `orchestrate_status`. Key discovery: `.assay/orchestrator/` must be gitignored to prevent state files from interfering with merge phase branch checkouts.

## Verification

- `cargo test -p assay-mcp` — 106 unit + 29 integration tests pass (11 new orchestrate tests)
- `cargo test -p assay-cli -- run` — 12 tests pass (8 new)
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — 3 tests pass
- `cargo test -p assay-mcp -- orchestrate_status` — 7 tests pass (5 unit + 2 integration)
- `just ready` — all checks pass (fmt, lint, test, deny)
- 1180 total workspace tests, all green

## Requirements Advanced

- R020 (Multi-agent orchestration) — CLI now routes multi-session manifests to orchestrator; end-to-end integration tests prove DAG → parallel execution → merge with real git repos
- R021 (Orchestration MCP tools) — `orchestrate_run` and `orchestrate_status` registered, tested, and returning structured results

## Requirements Validated

- R020 — Integration tests prove: 3-session DAG with dependencies executes in correct order, failure propagation skips dependents while continuing independent sessions, all successful branches merge into base. CLI routes correctly. MCP tool routes correctly.
- R021 — Both MCP tools registered in router (verified by registration tests), schemas generate correctly, handlers return structured JSON with orchestration outcomes + merge report, error paths return domain errors for missing manifests/run_ids. 13 tests total.

## New Requirements Surfaced

- R028 (candidate) — `.assay/orchestrator/` gitignore scaffolding: projects using orchestration need `.assay/.gitignore` with `orchestrator/` to prevent state files from interfering with merge-phase branch checkouts. Should be handled by `assay init` or first orchestrated run.

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T01: Task plan references `cargo test -p assay-mcp --features orchestrate` but assay-mcp has no `orchestrate` feature (it depends on assay-core with that feature enabled). Used `cargo test -p assay-mcp` instead.
- T03: Added `.assay/.gitignore` with `orchestrator/` in test setup — not in original plan but required to prevent merge runner's clean-worktree check from rejecting untracked orchestrator state directory.

## Known Limitations

- Orchestrated runs hardcode Claude Code adapter (D040) — per-session adapter selection requires a `harness` field on ManifestSession (future work).
- Real agent invocation in orchestrated runs is untested — integration tests use mock runners. Manual UAT with real agents required.
- `.assay/orchestrator/` gitignore is not automatically scaffolded — projects must add it manually or discover it when merge fails.

## Follow-ups

- Add `.assay/.gitignore` scaffolding to `assay init` or auto-create on first orchestrated run (surfaces R028)
- Per-session adapter selection via ManifestSession `harness` field (supersedes D040)
- Manual UAT: run a 3+ session manifest with real Claude Code agents against real specs

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` — Added orchestrate_run and orchestrate_status tools with param/response types and 11 unit tests
- `crates/assay-mcp/src/lib.rs` — Re-exported OrchestrateRunParams and OrchestrateStatusParams under testing cfg
- `crates/assay-cli/src/commands/run.rs` — Added --failure-policy/--merge-strategy flags, needs_orchestration() detection, execute_orchestrated() path, OrchestrationResponse types, 8 new tests
- `crates/assay-core/tests/orchestrate_integration.rs` — 3 integration tests with real git repos proving full DAG → execute → merge → status path
- `crates/assay-mcp/tests/mcp_handlers.rs` — 2 tests for orchestrate_status handler

## Forward Intelligence

### What the next slice should know
- All M002 slices are complete. The next work is milestone wrap-up (M002-SUMMARY) and then M003 planning.
- The orchestrator is fully wired but only tested with mock runners — real agent invocation is the remaining UAT gap.

### What's fragile
- `.assay/orchestrator/` gitignore handling — if a project doesn't have the gitignore, orchestrated merges will fail with clean-worktree errors that don't clearly point to the root cause.
- Base branch detection uses `git rev-parse --abbrev-ref HEAD` — detached HEAD state will produce unhelpful results.

### Authoritative diagnostics
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — if this passes, the full orchestration pipeline is healthy
- `.assay/orchestrator/<run_id>/state.json` — authoritative source of truth for orchestration state

### What assumptions changed
- Originally assumed `git add .` in test mock runners would be fine — actually stages `.assay/orchestrator/` state files causing checkout to delete them. Mock runners must `git add <specific-file>` instead.
