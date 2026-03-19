---
id: M002
provides:
  - "DAG-driven parallel session executor with std::thread::scope, bounded concurrency, and failure propagation"
  - "Sequential merge runner with topological ordering (CompletionTime + FileOverlap strategies)"
  - "Closure-based conflict handler contract with default skip behavior"
  - "Codex adapter (TOML config) and OpenCode adapter (JSON config) alongside existing Claude Code adapter"
  - "Harness CLI surface: assay harness generate|install|update|diff for all three adapters"
  - "Globset-based scope enforcement with multi-agent awareness prompt injection"
  - "orchestrate_run and orchestrate_status MCP tools (22 total)"
  - "CLI multi-session routing with --failure-policy and --merge-strategy flags"
  - "State persistence to .assay/orchestrator/<run_id>/state.json"
  - "ManifestSession.depends_on for inter-session dependency declaration"
key_decisions:
  - "D017: std::thread::scope with bounded concurrency — zero-dependency, sync-core convention"
  - "D019: Topological-order sequential merge with re-check before each merge"
  - "D024: Hand-rolled Kahn's algorithm — no petgraph dependency for ≤20-node graphs"
  - "D026: Closure-based conflict handler (not trait), AI resolution deferred to M003"
  - "D031: Two-phase pipeline split (setup_session + execute_session) for worktree serialization"
  - "D034: Generic F: Fn + Sync for session runner instead of dyn trait object"
  - "D037: Scope prompt injected as PromptLayer, not by modifying adapter signatures"
  - "D039: Multi-session detection heuristic (sessions.len() > 1 OR any depends_on)"
  - "D041: .assay/orchestrator/ must be gitignored to prevent state file interference"
patterns_established:
  - "Feature gate pattern: orchestrate feature on assay-core and assay-types, enabled by downstream crates"
  - "Condvar-based dispatch loop: outer loop acquires batch via ready_set(), spawns scoped threads"
  - "Integration test pattern: tempdir + git init + .assay setup + mock runner with real branches/commits"
  - "GeneratedConfig enum wraps adapter-specific config with unified files()/write() interface"
  - "Scope prompt injection at call site via PromptLayer — keeps adapters pure"
  - "Three-level deterministic tiebreaking for merge ordering (primary metric, topo_order, session_name)"
observability_surfaces:
  - ".assay/orchestrator/<run_id>/state.json — OrchestratorStatus with per-session state, timing, phase"
  - "orchestrate_status MCP tool reads persisted state without holding executor reference"
  - "CLI stderr phase markers (Phase 1/2/3) and --json for OrchestrationResponse"
  - "MergeReport with per-session merge status, ordering plan, and aggregate counts"
  - "assay harness diff exit code 0/1 for change detection"
requirement_outcomes:
  - id: R020
    from_status: active
    to_status: validated
    proof: "S06 integration tests prove 3-session DAG with dependencies executes in correct order, failure propagation skips dependents, CLI and MCP route correctly. 3 integration tests with real git repos + 8 CLI tests + 11 MCP tests."
  - id: R021
    from_status: active
    to_status: validated
    proof: "S06 — orchestrate_run and orchestrate_status registered (22 total tools), schema tests, param deserialization, handler tests. 13 total tests (11 unit + 2 integration)."
  - id: R022
    from_status: active
    to_status: validated
    proof: "S05 — check_scope() with globset patterns (9 tests), generate_scope_prompt() multi-agent markdown, CLI generate/install/update/diff for all three adapters (11 tests), ScopeViolation types with schema snapshots."
  - id: R023
    from_status: active
    to_status: validated
    proof: "S03 — merge_completed_sessions() with CompletionTime/FileOverlap strategies, closure-based conflict handler, pre-flight validation. 21 new tests with real git repos. 10 schema snapshots."
  - id: R024
    from_status: active
    to_status: validated
    proof: "S04 — Codex adapter (12 tests, 9 snapshots), OpenCode adapter (10 tests, 9 snapshots). Both follow Claude adapter pattern. 49 total harness tests, 30 snapshots."
duration: "~4 hours across 6 slices (20 tasks)"
verification_result: passed
completed_at: 2026-03-17
---

# M002: Multi-Agent Orchestration & Harness Platform

**Transformed Assay from single-agent pipeline to multi-agent orchestration platform: DAG-driven parallel execution with bounded concurrency, sequential merge with ordering strategies, three harness adapters with scope enforcement, and full CLI + MCP surfaces — proven by 1180 tests including end-to-end integration with real git repos.**

## What Happened

M002 delivered in 6 slices across ~4 hours, porting Smelt's battle-tested orchestration concepts to Assay's sync/closure conventions.

**Foundation (S01):** Extended `ManifestSession` with `depends_on: Vec<String>` and built `DependencyGraph` — a hand-rolled Kahn's algorithm over `Vec<Vec<usize>>` adjacency lists with cycle detection, missing-reference validation, and three query methods (`ready_set`, `mark_skipped_dependents`, `topological_groups`). Feature-gated behind `orchestrate` on assay-core and assay-types. 35 DAG tests.

**Parallel Executor (S02):** Built `run_orchestrated()` — a `std::thread::scope` dispatch loop using `Mutex + Condvar` that computes ready sets from the DAG, spawns bounded worker threads (default: min(sessions, 8)), records outcomes, propagates failures via BFS skip marking, and persists `OrchestratorStatus` to disk after each resolution. The pipeline was split into `setup_session()` (worktree creation, serialized via mutex) and `execute_session()` (agent launch, parallelized). Session runner is a generic `F: Fn + Sync` parameter for testability. 18 executor tests including diamond DAGs, abort policy, panic recovery, and bounded concurrency proofs.

**Merge Runner (S03):** Implemented `merge_execute()` for `git merge --no-ff` with structured conflict detection, `order_sessions()` with CompletionTime and FileOverlap strategies (three-level deterministic tiebreaking), and `merge_completed_sessions()` — the sequencing loop with closure-based conflict handler, pre-flight validation (clean tree + no MERGE_HEAD), and abort propagation. Default handler returns Skip. 21 tests with real git repos.

**Multi-Adapter Harness (S04):** Created Codex adapter (TOML config with sandbox mode escalation) and OpenCode adapter (JSON config with $schema field), both following the Claude Code adapter pattern from M001. Hook support is advisory-only (markdown in AGENTS.md) since neither agent has native hook lifecycle. 22 new tests, 18 new snapshots.

**Harness CLI & Scope (S05):** Built `assay harness generate|install|update|diff` dispatching to all three adapters via `GeneratedConfig` enum. Implemented globset-based `check_scope()` returning `Vec<ScopeViolation>` and `generate_scope_prompt()` producing multi-agent awareness markdown injected as `PromptLayer` (priority -100) before adapter dispatch. Added `file_scope` and `shared_files` fields to ManifestSession. 22 new tests.

**End-to-End Integration (S06):** Wired everything together: `orchestrate_run` and `orchestrate_status` MCP tools (22 total), CLI multi-session routing with `--failure-policy` and `--merge-strategy` flags, and 3 end-to-end integration tests with real git repos proving the full path (DAG validation → parallel execution → sequential merge → status persistence). Discovered that `.assay/orchestrator/` must be gitignored to prevent state files from interfering with merge-phase branch checkouts (D041).

## Cross-Slice Verification

**Multi-session DAG → parallel execution → merge:** S06 integration test #1 creates a 3-session DAG (A→B dependency, C independent), executes with mock runners creating real git branches/commits, and merges all into base. Proves correct dependency ordering and parallel execution.

**Failure propagation:** S06 integration test #2 fails session A, verifies B is skipped (dependent), C succeeds and merges alone. Proves `SkipDependents` policy works end-to-end.

**Status persistence round-trip:** S06 integration test #3 verifies all `OrchestratorStatus` fields after a run, confirming MCP `orchestrate_status` can read persisted state.

**Backward compatibility:** S02 `single_session_compat` test runs a single-session manifest through the orchestrator. S06 `needs_orchestration()` ensures single-session manifests without `depends_on` route to the existing `run_manifest()` path.

**Harness adapters:** S04 locks all three adapters with 30 insta snapshots. S05 CLI tests verify dispatch to each adapter via `assay harness generate claude-code|codex|opencode`.

**Scope enforcement:** S05 tests verify globset pattern matching for out-of-scope and shared-file violations, and scope prompt injection as PromptLayer.

**Conflict handler contract:** S03 integration tests exercise the closure-based handler with both Skip and Abort actions on real merge conflicts.

**`just ready` passes:** All 1180 tests green, clippy clean, fmt clean, deny clean.

## Requirement Changes

- R020: active → validated — End-to-end integration tests prove DAG-driven parallel execution with dependency ordering, failure propagation, and bounded concurrency through both CLI and MCP paths
- R021: active → validated — orchestrate_run and orchestrate_status MCP tools registered (22 total), tested with 13 tests covering schemas, param handling, error paths, and status reads
- R022: active → validated — Globset-based scope enforcement with multi-agent prompt generation, CLI harness generate/install/update/diff for all three adapters, 22 new tests
- R023: active → validated — Sequential merge runner with topological ordering, CompletionTime/FileOverlap strategies, closure-based conflict handler, 21 tests with real git repos
- R024: active → validated — Codex and OpenCode adapters generating valid config from HarnessProfile, locked by 30 total snapshots across all three adapters

## Forward Intelligence

### What the next milestone should know
- The orchestrator is fully wired but only tested with mock runners — real agent invocation (Claude Code, Codex, OpenCode) is the remaining UAT gap. Manual testing with real agents should be the first M003 activity.
- `run_orchestrated()` is generic over the session runner closure — swapping in a real two-phase runner (setup under mutex, execute in parallel) requires composing `setup_session()` + `execute_session()` at the call site.
- The conflict handler contract is `Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction` — M003's AI conflict resolution plugs in here by providing a closure that invokes an evaluator agent.
- `extract_completed_sessions()` derives branch names from session names via `assay/<slug>` when `branch_name` is empty — production use should populate real branch names from executor worktree metadata.

### What's fragile
- `.assay/orchestrator/` gitignore handling — if a project doesn't have the gitignore entry, orchestrated merges fail with clean-worktree errors that don't clearly point to the root cause. Should be auto-scaffolded by `assay init` or first orchestrated run.
- Branch name derivation in `extract_completed_sessions()` uses simple slug from session name — unusual characters in session names may produce mismatched branch names.
- OpenCode and Codex config formats are based on research, not official schema validation — format changes in those tools will require snapshot updates.
- The condvar dispatch loop has a spurious wakeup guard (inner loop) that's critical — removing it causes busy-wait.
- `ready_set()` requires `completed ∪ failed` in its `completed` parameter — passing only `completed` causes failed sessions to be re-dispatched.

### Authoritative diagnostics
- `cargo test -p assay-core --features orchestrate --test orchestrate_integration` — if this passes, the full orchestration pipeline is healthy
- `.assay/orchestrator/<run_id>/state.json` — single source of truth for orchestration state
- `crates/assay-harness/src/snapshots/` — 30 snapshot files lock all adapter output formats; any mismatch produces an inline diff
- `MergeReport` — single source of truth for merge outcomes with per-session status and aggregate counts

### What assumptions changed
- Assumed `dyn Fn` would work for session runner — actually needs generic `F: Fn + Sync` due to `'static` lifetime requirement on trait objects (D034)
- Assumed `HarnessWriter` could be passed directly to `run_orchestrated()` — actually not `Sync`, must be captured in closure (D035)
- Assumed `git add .` in test mock runners would be fine — actually stages `.assay/orchestrator/` state files on branches, breaking checkout. Mock runners must `git add <specific-file>` (D041)

## Files Created/Modified

- `crates/assay-types/src/manifest.rs` — depends_on, file_scope, shared_files fields on ManifestSession
- `crates/assay-types/src/orchestrate.rs` — SessionRunState, FailurePolicy, OrchestratorPhase, SessionStatus, OrchestratorStatus, MergeStrategy, MergePlan, MergePlanEntry, MergeSessionStatus, MergeSessionResult, MergeReport, ConflictAction
- `crates/assay-types/src/merge.rs` — MergeExecuteResult, ConflictScan, ConflictMarker, MarkerType
- `crates/assay-types/src/harness.rs` — ScopeViolationType, ScopeViolation
- `crates/assay-core/src/orchestrate/mod.rs` — module root (dag, executor, ordering, merge_runner)
- `crates/assay-core/src/orchestrate/dag.rs` — DependencyGraph with from_manifest(), ready_set(), mark_skipped_dependents(), topological_groups()
- `crates/assay-core/src/orchestrate/executor.rs` — run_orchestrated() parallel executor
- `crates/assay-core/src/orchestrate/ordering.rs` — CompletedSession, order_sessions() with CompletionTime/FileOverlap
- `crates/assay-core/src/orchestrate/merge_runner.rs` — merge_completed_sessions(), default_conflict_handler(), extract_completed_sessions()
- `crates/assay-core/src/merge.rs` — merge_execute(), scan_conflict_markers(), scan_files_for_markers()
- `crates/assay-core/src/pipeline.rs` — setup_session(), execute_session(), SetupResult
- `crates/assay-core/src/error.rs` — DagCycle, DagValidation, MergeExecuteError, MergeRunnerError variants
- `crates/assay-core/tests/orchestrate_integration.rs` — 3 end-to-end integration tests
- `crates/assay-harness/src/codex.rs` — Codex adapter (generate_config, write_config, build_cli_args)
- `crates/assay-harness/src/opencode.rs` — OpenCode adapter (generate_config, write_config, build_cli_args)
- `crates/assay-harness/src/scope.rs` — check_scope(), generate_scope_prompt()
- `crates/assay-cli/src/commands/harness.rs` — HarnessCommand with generate/install/update/diff
- `crates/assay-cli/src/commands/run.rs` — --failure-policy, --merge-strategy, needs_orchestration(), execute_orchestrated()
- `crates/assay-mcp/src/server.rs` — orchestrate_run and orchestrate_status MCP tools
- `crates/assay-mcp/tests/mcp_handlers.rs` — orchestrate_status handler tests
- `crates/assay-types/tests/snapshots/` — 17 new schema snapshot files
- `crates/assay-harness/src/snapshots/` — 18 new adapter snapshot files
