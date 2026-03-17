# M002: Multi-Agent Orchestration & Harness Platform

**Vision:** Transform Assay into the complete agent development platform by absorbing Smelt's orchestration capabilities and shipping the full harness CLI surface. A user writes a multi-session TOML manifest with dependency declarations, and `assay run` orchestrates everything: DAG-ordered parallel execution across isolated worktrees, scope-enforced harness config generation for any supported agent (Claude Code, Codex, OpenCode), sequential merge with conflict detection, and live status reporting. Externally, Smelt calls `assay harness generate` to configure agents in provisioned environments — Assay becomes the harness layer, Smelt becomes the infra layer.

## Success Criteria

- A multi-session manifest with `depends_on` declarations launches agents in correct dependency order, running independent sessions concurrently
- If one agent fails, its dependents are skipped but independent sessions continue to completion
- Completed session branches merge into the base branch in topological order using `git merge --no-ff`, with file-overlap ordering available to minimize conflicts
- Each agent's generated harness config includes scope boundaries (which files it owns, which it must not touch) and multi-agent awareness
- `assay harness generate claude-code|codex|opencode --spec <name>` produces valid harness-specific config files from a HarnessProfile
- `assay harness install|update|diff` manages harness config lifecycle in a project
- `orchestrate_status` MCP tool shows real-time state of each session (waiting, running, completed, failed, skipped)
- Existing single-session manifests continue to work without modification
- Conflict handler contract exists (closure-based), with a noop/skip default — AI resolution deferred to M003

## Key Risks / Unknowns

- Porting Smelt's async/trait-based orchestrator to Assay's sync/closure conventions may surface design friction — Smelt uses `tokio::task::JoinSet` and `trait ConflictHandler`, Assay requires `std::thread::scope` and closures
- Concurrent git worktree creation may race on `.git/worktrees/` directory — Smelt serializes worktree creation but parallelizes agent execution
- Sequential merge ordering is sensitive — merging A then B may succeed but B then A may conflict; Smelt's file-overlap strategy mitigates but doesn't eliminate this
- Codex and OpenCode adapter fidelity — need to research their actual config formats (Claude Code adapter is proven from M001)

## Proof Strategy

- Sync orchestration feasibility → retire in S02 by running real parallel sessions via `std::thread::scope` with correct DAG ordering and failure propagation
- Merge ordering correctness → retire in S03 by executing sequential merge in topological order with file-overlap strategy, verified by integration tests with real git repos
- Multi-adapter harness generation → retire in S04 by generating valid config for all three harness targets from the same HarnessProfile

## Verification Classes

- Contract verification: `just ready` (fmt, lint, test, deny), schema snapshots for new types, round-trip tests for manifest extensions, DAG cycle detection tests, merge ordering tests, adapter snapshot tests
- Integration verification: multiple concurrent `run_session` invocations on real worktrees, actual `git merge --no-ff` execution, real concurrent git operations, harness config validated against target agent's expected format
- Operational verification: orchestrator handles agent timeout/crash mid-run, partial completion recovery, bounded concurrency enforcement, conflict handler invocation on merge conflicts
- UAT / human verification: run a multi-session manifest against real specs with real agents; run `assay harness generate` for each adapter and inspect output; run `assay harness install` and verify project config

## Milestone Definition of Done

This milestone is complete only when all are true:

- All slices are complete with passing verification
- A multi-session manifest with dependencies launches parallel agents, skips dependents of failures, and merges results in topological order
- Single-session manifests still work identically to M001 behavior
- `assay harness generate claude-code|codex|opencode` produces valid output
- `assay harness install|update|diff` manages config lifecycle for at least Claude Code
- `just ready` passes on main after all slices are squash-merged
- The real CLI entrypoint (`assay run`) routes multi-session manifests to the orchestrator
- Orchestration status is observable through MCP tools
- Final integrated acceptance: 3+ session manifest with mixed dependencies, one intentional failure, verified correct skip/continue/merge behavior through the real CLI

## Requirement Coverage

- Covers: R020, R021, R022, R023, R024
- Partially covers: none
- Leaves for later: R025 (SessionCore unification), R026 (AI conflict resolution)
- Orphan risks: none

## Slices

- [x] **S01: Manifest Dependencies & DAG Validation** `risk:high` `depends:[]`
  > After this: user authors a multi-session manifest with `depends_on` fields. `assay run` validates the dependency graph, rejects cycles and missing references with actionable error messages, and shows the execution plan (topological order with parallelism groups). Verified by unit tests and a real multi-session manifest parsed from disk. Reference spec: Smelt's `orchestrate/dag.rs` (ready_set, mark_skipped_dependents semantics).

- [x] **S02: Parallel Session Executor** `risk:high` `depends:[S01]`
  > After this: `assay run` on a multi-session manifest launches independent sessions concurrently via `std::thread::scope` with bounded concurrency (default: min(sessions, 8)), serializes worktree creation through a mutex, respects dependency ordering, skips dependents of failed sessions, and reports per-session outcomes with timing. Orchestrator state persists to disk for status queries. Verified by unit tests with mock harness writers confirming parallel execution, correct ordering, and failure propagation. Reference spec: Smelt's `orchestrate/executor.rs` (phases, failure policy, state persistence).

- [x] **S03: Sequential Merge Runner & Conflict Contract** `risk:medium` `depends:[S02]`
  > After this: after parallel execution completes, the orchestrator merges each successful session's branch into the base branch in topological order using `git merge --no-ff`. Merge ordering supports completion-time (default) and file-overlap strategies. Each merge re-checks for conflicts against the updated base. Conflict handler is a closure receiving (session_name, files, scan, work_dir) → Resolved/Skip/Abort. Default handler skips on conflict. Failed merges reported with conflicting files. Verified by integration tests with real git repos containing parallel branches. Reference spec: Smelt's `merge/` module (ordering.rs, conflict.rs, mod.rs squash-merge loop).

- [x] **S04: Codex & OpenCode Adapters** `risk:medium` `depends:[]`
  > After this: `assay harness generate codex --spec auth` and `assay harness generate opencode --spec auth` produce valid harness-specific config from a HarnessProfile, following the same adapter pattern as Claude Code (M001/S04). Each adapter generates the target agent's config format (instructions file, settings, tool permissions). Verified by snapshot tests and config structure assertions. R024 delivered.

- [ ] **S05: Harness CLI & Scope Enforcement** `risk:low` `depends:[S04,S02]`
  > After this: `assay harness generate claude-code|codex|opencode [--spec <name>] [--workflow <phase>]` generates harness config to stdout or disk. `assay harness install <adapter>` writes config into the project. `assay harness update <adapter>` applies incremental changes. `assay harness diff <adapter>` shows what would change without applying. Each session's generated config includes scope boundaries (file_scope + shared_files via globset matching) and multi-agent awareness prompts. Verified by CLI integration tests and scope violation detection tests. Reference spec: Smelt's `summary/scope.rs` (globset matching, ScopeViolation).

- [ ] **S06: MCP Tools & End-to-End Integration** `risk:high` `depends:[S03,S05]`
  > After this: `orchestrate_run` MCP tool launches multi-session orchestration. `orchestrate_status` MCP tool returns live session states (waiting/running/completed/failed/skipped). `assay run <manifest.toml>` detects single vs multi-session manifests and routes accordingly. A 3+ session manifest with mixed dependencies exercises the full path: DAG validation → parallel execution → scope-enforced harness config → sequential merge with ordering → status reporting. Pipeline failures at any stage produce structured errors with orchestration context. This is proven by integration tests exercising the real entrypoint with concurrent sessions, intentional failures, and merge ordering — not just by assembling previously-tested components. `just ready` passes with all new code integrated.

## Boundary Map

### S01 → S02

Produces:
- `assay-types/src/manifest.rs` → `ManifestSession.depends_on: Vec<String>` field (`#[serde(default)]` for backward compat)
- `assay-core/src/orchestrate/dag.rs` → `DependencyGraph` struct with adjacency list representation
- `assay-core/src/orchestrate/dag.rs` → `DependencyGraph::from_manifest(&RunManifest) -> Result<DependencyGraph>` — builds graph, validates (cycles via Kahn's, missing refs, duplicate specs)
- `assay-core/src/orchestrate/dag.rs` → `DependencyGraph::topological_groups(&self) -> Vec<Vec<usize>>` — parallelism groups where each inner Vec can run concurrently
- `assay-core/src/orchestrate/dag.rs` → `DependencyGraph::ready_set(&self, completed, in_flight, skipped) -> Vec<usize>` — sessions ready to dispatch
- `assay-core/src/orchestrate/dag.rs` → `DependencyGraph::mark_skipped_dependents(&self, failed, skipped) -> ()` — BFS marks transitive dependents
- `assay-core/src/orchestrate/mod.rs` → module root behind `cfg(feature = "orchestrate")`
- `assay-core/Cargo.toml` → `orchestrate` feature gate

Consumes:
- nothing (first slice; extends existing `RunManifest`/`ManifestSession` types from M001)

### S01 → S03

Produces:
- `DependencyGraph::topological_groups()` — merge runner needs topological order for merge sequencing

### S01 → S05

Produces:
- `ManifestSession.depends_on` field — scope enforcement needs to know session relationships

### S01 → S06

Produces:
- DAG validation — CLI routes to orchestrator when manifest has multiple sessions or dependencies

### S02 → S03

Produces:
- `assay-core/src/orchestrate/executor.rs` → `run_orchestrated(manifest, config, harness_writer) -> OrchestratorResult`
- `assay-core/src/orchestrate/executor.rs` → `SessionOutcome` enum: Completed { worktree_path, branch_name, changed_files }, Failed { stage, message, recovery }, Skipped { reason }
- `assay-core/src/orchestrate/executor.rs` → `OrchestratorResult` struct: outcomes per session, timing, failure policy applied
- `assay-types/src/orchestrate.rs` → `OrchestratorStatus` (serializable snapshot: run_id, phase, per-session SessionRunState, timing)
- `assay-types/src/orchestrate.rs` → `SessionRunState` enum: Pending, Running, Completed, Failed, Skipped, Cancelled
- `assay-types/src/orchestrate.rs` → `FailurePolicy` enum: SkipDependents (default), Abort
- State persistence: `.assay/orchestrator/<run_id>/state.json`

Consumes from S01:
- `DependencyGraph` with `ready_set()` for dispatch scheduling
- `mark_skipped_dependents()` for failure propagation
- `ManifestSession.depends_on` for building the graph

### S02 → S04

Produces:
- Nothing directly — S04 is independent. But the executor will call harness adapters via the `HarnessWriter` closure at runtime.

### S02 → S05

Produces:
- `OrchestratorResult` and session list — scope enforcement needs session context for multi-agent prompts

### S02 → S06

Produces:
- `run_orchestrated()` as the core execution engine
- `OrchestratorStatus` for MCP status tool

### S03 → S06

Produces:
- `assay-core/src/merge.rs` → `merge_execute(project_root, worktree_branch, base_branch) -> Result<MergeExecuteResult>`
- `assay-core/src/orchestrate/merge_runner.rs` → `merge_completed_sessions(outcomes, config, conflict_handler) -> MergeReport`
- `assay-core/src/orchestrate/merge_runner.rs` → `ConflictAction` enum: Resolved(ResolutionMethod), Skip, Abort
- `assay-core/src/orchestrate/merge_runner.rs` → `MergeReport` struct: sessions_merged, sessions_skipped, sessions_conflict_skipped, plan, totals
- `assay-core/src/orchestrate/ordering.rs` → `order_sessions(sessions, strategy) -> (Vec<CompletedSession>, MergePlan)`
- `assay-core/src/merge.rs` → `scan_conflict_markers(content) -> ConflictScan` and `scan_files_for_markers(dir, files) -> ConflictScan`

Consumes from S02:
- `OrchestratorResult` with `SessionOutcome::Completed` entries — knows which branches to merge and their changed files

### S04 → S05

Produces:
- `assay-harness/src/codex.rs` → `generate_config(profile: &HarnessProfile, worktree: &Path) -> Result<CodexConfig>`
- `assay-harness/src/codex.rs` → `write_config(config: &CodexConfig, worktree: &Path) -> Result<()>`
- `assay-harness/src/opencode.rs` → `generate_config(profile: &HarnessProfile, worktree: &Path) -> Result<OpenCodeConfig>`
- `assay-harness/src/opencode.rs` → `write_config(config: &OpenCodeConfig, worktree: &Path) -> Result<()>`
- All three adapters (claude, codex, opencode) share the same HarnessProfile input contract

Consumes:
- nothing (independent slice; uses existing HarnessProfile + adapter pattern from M001/S04)

### S05 → S06

Produces:
- `assay-cli` → `assay harness generate|install|update|diff` subcommands
- `assay-core/src/orchestrate/scope.rs` → `check_scope(session_name, file_scope, shared_files, changed_files) -> Vec<ScopeViolation>`
- `assay-harness/src/scope.rs` → `generate_scope_prompt(session, all_sessions) -> String` — multi-agent awareness prompt section
- Updated adapter `generate_config()` functions to accept optional scope/multi-agent context

Consumes from S04:
- All three adapter generate/write functions for CLI dispatch
Consumes from S02:
- Session list and dependency context for multi-agent prompt generation

### S06 (capstone)

Consumes from S03:
- `merge_completed_sessions()` for the merge phase after parallel execution
- `merge_execute()` for actual git merge operations
- Conflict handler closure contract
Consumes from S05:
- Harness CLI (integration test verifies CLI path)
- Scope-aware config generation
Consumes from S01:
- DAG validation (CLI routes multi-session manifests to orchestrator)
Consumes from S02:
- `run_orchestrated()` as core execution engine
- `OrchestratorStatus` for MCP status tool
