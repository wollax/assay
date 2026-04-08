# Roadmap: Assay

## Milestones

<details>
<summary>✅ v0.1.0 Proof of Concept — SHIPPED 2026-03-02</summary>

**Goal:** Prove Assay's dual-track gate differentiator through a thin vertical slice — foundation types, spec-driven gates, MCP server, and Claude Code plugin.

- [x] Phase 1: Workspace Prerequisites (1 plan) — 2026-02-28
- [x] Phase 2: MCP Spike (1 plan) — 2026-02-28
- [x] Phase 3: Error Types and Domain Model (2 plans) — 2026-02-28
- [x] Phase 4: Schema Generation (1 plan) — 2026-02-28
- [x] Phase 5: Config and Initialization (3 plans) — 2026-03-01
- [x] Phase 6: Spec Files (2 plans) — 2026-03-01
- [x] Phase 7: Gate Evaluation (2 plans) — 2026-03-01
- [x] Phase 8: MCP Server Tools (2 plans) — 2026-03-01
- [x] Phase 9: CLI Surface Completion (2 plans) — 2026-03-02
- [x] Phase 10: Claude Code Plugin (2 plans) — 2026-03-02

[Full archive](milestones/v0.1.0-ROADMAP.md)

</details>

<details>
<summary>✅ v0.2.0 Dual-Track Gates & Hardening — SHIPPED 2026-03-08</summary>

**Goal:** Ship agent-evaluated gates, run history persistence, enforcement levels, session diagnostics, team context protection, and comprehensive hardening.

- [x] Phase 11: Type System Foundation (2 plans) — 2026-03-04
- [x] Phase 12: FileExists Gate Wiring (1 plan) — 2026-03-04
- [x] Phase 13: Enforcement Levels (3 plans) — 2026-03-04
- [x] Phase 14: Run History Core (2 plans) — 2026-03-05
- [x] Phase 15: Run History CLI (2 plans) — 2026-03-05
- [x] Phase 16: Agent Gate Recording (4 plans) — 2026-03-05
- [x] Phase 17: MCP Hardening & Agent History (2 plans) — 2026-03-05
- [x] Phase 18: CLI Hardening & Enforcement Surface (2 plans) — 2026-03-05
- [x] Phase 19: Testing & Tooling (3 plans) — 2026-03-06
- [x] Phase 20: Session JSONL Parser & Token Diagnostics (5 plans) — 2026-03-06
- [x] Phase 21: Team State Checkpointing (3 plans) — 2026-03-06
- [x] Phase 22: Pruning Engine (5 plans) — 2026-03-06
- [x] Phase 23: Guard Daemon & Recovery (4 plans) — 2026-03-07
- [x] Phase 24: MCP History Persistence Fix (1 plan) — 2026-03-07
- [x] Phase 25: Tech Debt Cleanup (2 plans) — 2026-03-07

[Full archive](milestones/v0.2.0-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3.0 Orchestration Foundation — SHIPPED 2026-03-10</summary>

**Goal:** Build the foundation for agent orchestration — worktree isolation, independent gate evaluation infrastructure, and CLI/MCP/types/core hardening — while closing tech debt from v0.2.0.

- [x] Phase 26: Structural Prerequisites (2 plans) — 2026-03-09
- [x] Phase 27: Types Hygiene (4 plans) — 2026-03-09
- [x] Phase 28: Worktree Manager (2 plans) — 2026-03-09
- [x] Phase 29: Gate Output Truncation (2 plans) — 2026-03-09
- [x] Phase 30: Core Tech Debt (3 plans) — 2026-03-10
- [x] Phase 31: Error Messages (2 plans) — 2026-03-10
- [x] Phase 32: CLI Polish (4 plans) — 2026-03-10
- [x] Phase 33: MCP Validation (2 plans) — 2026-03-10
- [x] Phase 34: MCP Truncation Visibility (1 plan) — 2026-03-10

[Full archive](milestones/v0.3.0-ROADMAP.md)

</details>

<details>
<summary>✅ v0.4.0 Headless Orchestration — SHIPPED 2026-03-15</summary>

**Goal:** Ship `gate_evaluate` as the capstone MCP tool — agent-evaluated gates driven by headless subprocess orchestration, backed by session persistence, context-aware diff budgeting, spec validation, and observability improvements.

- [x] Phase 35: Observability Foundation (2 plans) — 2026-03-11
- [x] Phase 36: Correctness & Robustness (3 plans) — 2026-03-11
- [x] Phase 37: Spec Validation (2 plans) — 2026-03-11
- [x] Phase 38: Observability Completion (2 plans) — 2026-03-13
- [x] Phase 39: Context Engine Integration (2 plans) — 2026-03-15
- [x] Phase 40: WorkSession Type & Persistence (2 plans) — 2026-03-15
- [x] Phase 41: Session MCP Tools (1 plan) — 2026-03-15
- [x] Phase 42: Session Recovery & Internal API (2 plans) — 2026-03-15
- [x] Phase 43: gate_evaluate Schema & Subprocess (2 plans) — 2026-03-15
- [x] Phase 44: gate_evaluate Context Budgeting (2 plans) — 2026-03-15
- [x] Phase 45: Tech Debt Cleanup (9 plans) — 2026-03-15

[Full archive](milestones/v0.4.0-ROADMAP.md)

</details>

### ○ v0.4.1 Merge Tools

**Goal:** Ship merge conflict detection and PR-based merge proposal as MCP tools — enabling agents to safely check for conflicts and propose merges through pull requests with gate evidence, backed by forge-agnostic env vars and worktree fixes.

- [x] Phase 46: Worktree Fixes (3 plans, 3 waves — sequential, shared file) — 2026-03-16
  - WFIX-01: Cleanup `--all` uses canonical path from git (Plan 01, Wave 1)
  - WFIX-02: Default branch detection provides actionable error (Plan 02, Wave 2)
  - WFIX-03: Prune failures surfaced as warnings (Plan 03, Wave 3)

- [x] Phase 47: Merge Check (2 plans, 2 waves — sequential) — 2026-03-16
  - MERGE-01: `merge_check` MCP tool
  - Plan 01 (Wave 1): Types in assay-types + core logic in assay-core (merge_check function, git CLI orchestration, conflict parsing)
  - Plan 02 (Wave 2): MCP tool wiring on AssayServer + handler tests
  - **Goal:** Standalone conflict detection via `git merge-tree --write-tree` with zero side effects
  - **Success criteria:**
    1. `merge_check` MCP tool accepts `base` and `head` refs and returns `MergeCheck { clean, conflicts, base_sha, head_sha }`
    2. Uses `git merge-tree --write-tree` (Git 2.38+) — no index mutation, no working tree changes
    3. When merge is clean, `conflicts` is empty and `clean` is true
    4. When merge has conflicts, `conflicts` lists affected file paths and `clean` is false

- [x] Phase 48: Gate Evidence Formatting — 2026-03-16
  - MERGE-04: Gate evidence formatting for PR body
  - **Goal:** Format gate results as markdown suitable for PR bodies with GitHub character limit handling
  - **Success criteria:**
    1. Gate results are formatted as markdown with per-criterion pass/fail status and evaluator reasoning
    2. PR body is truncated at 65,536 chars with a link to the full gate report path
    3. Truncation preserves the summary section and truncates individual criterion details
    4. When gate results fit within limit, full content is included without truncation markers
  - **Plans:**
    - Plan 01 (Wave 1): FormattedEvidence type + format_gate_evidence() with semantic truncation + save_report()
    - Plan 02 (Wave 2): Comprehensive test coverage — formatting variants, truncation edge cases, persistence

- [x] Phase 49: Forge-Agnostic Env Vars — 2026-04-08
  - MERGE-05: Forge-agnostic extensibility via env vars
  - **Goal:** Set env vars for downstream tooling and validate `gh` CLI availability
  - **Success criteria:**
    1. `merge_propose` sets `$ASSAY_BRANCH`, `$ASSAY_SPEC`, and `$ASSAY_GATE_REPORT_PATH` env vars before invoking forge CLI
    2. Clear error message when `gh` CLI is not found on PATH, naming the dependency and linking to installation docs
    3. Env vars are documented in MCP tool schema descriptions

- [ ] Phase 50: Merge Propose
  - MERGE-02: `merge_propose` MCP tool
  - MERGE-03: `dry_run: bool` parameter
  - **Goal:** Push branch and create PR with gate evidence — the agent's path to merging work
  - **Success criteria:**
    1. `merge_propose` pushes worktree branch to remote and creates PR via `gh pr create`, returning `MergeProposal { pr_url, pr_number, gate_summary, dry_run }`
    2. PR body includes formatted gate evidence from Phase 48
    3. `dry_run: true` previews the PR (branch, title, body) without pushing or creating — returns the same `MergeProposal` shape with `dry_run: true` and no `pr_url`/`pr_number`
    4. Push-to-remote is documented as a side effect in the MCP tool schema description

### v0.5.0 Single-Agent Harness End-to-End

**Goal:** Ship the complete single-agent pipeline — from declarative RunManifest through worktree isolation, harness-driven agent launch, gate evaluation, to merge proposal — proving Assay can orchestrate one agent end-to-end with callback-based control inversion and the Claude Code adapter.

- [ ] Phase 51: Session Vocabulary Cleanup
  - PREREQ-02: Rename AgentSession to GateEvalContext, manifest to RunManifest, runner to RunExecutor
  - **Goal:** Eliminate overloaded session terminology before new types are added — prevents confusion between GateEvalContext (gate evaluation), WorkSession (worktree lifecycle), and future OrchestratorSession
  - **Success criteria:**
    1. `AgentSession` is renamed to `GateEvalContext` across assay-types and assay-mcp — all references updated, no dead aliases
    2. Smelt-originated `manifest` concept is renamed to `RunManifest` in all types and documentation
    3. Smelt-originated `runner` concept is renamed to `RunExecutor` in all types and documentation
    4. All existing tests pass with zero behavioral changes — rename is purely cosmetic

- [ ] Phase 52: Session Persistence
  - PREREQ-01: GateEvalContext write-through cache surviving MCP server restarts
  - **Goal:** Gate evaluation sessions persist to disk so MCP server restarts don't lose active evaluations — prerequisite for reliable multi-step pipelines
  - **Success criteria:**
    1. `GateEvalContext` writes to disk on every state mutation (write-through, not write-back)
    2. On MCP server startup, active sessions are recovered from disk without manual intervention
    3. A session created, mutated, then recovered after simulated restart retains all state (round-trip test)
    4. Concurrent write attempts to the same session file produce consistent results (no partial writes via atomic rename)

- [ ] Phase 53: Worktree Session Linkage
  - WTREE-01: Orphan detection for worktrees with no active session
  - WTREE-02: Collision prevention for duplicate active worktrees per spec
  - WTREE-03: WorktreeMetadata includes `session_id: Option<String>`
  - **Goal:** Link worktrees to sessions so orphaned worktrees are detectable and duplicate active worktrees per spec are prevented — foundation for reliable pipeline orchestration
  - **Success criteria:**
    1. `WorktreeMetadata` includes `session_id: Option<String>` field, serialized to worktree metadata JSON
    2. `worktree_list` or `worktree_status` identifies orphaned worktrees (worktrees where linked session_id does not exist or session is in terminal state)
    3. `worktree_create` rejects creation when the spec already has an active worktree with an in-progress session — returns actionable error naming the existing worktree and session
    4. Worktrees with `session_id: None` (pre-linkage) are treated as unlinked, not orphaned — backward compatible

- [ ] Phase 54: Worktree Tech Debt Batch
  - WTREE-04: 15 worktree tech debt issues resolved
  - **Goal:** Clean up accumulated worktree tech debt in a single batch — error chains, type hygiene, missing tests, serialization fixes, and schema registration
  - **Success criteria:**
    1. CLI worktree handlers preserve error source chains (no `.to_string()` on errors that implement `std::error::Error`)
    2. `WorktreeConfig.base_dir` uses `Option<PathBuf>` instead of `String`, `detect_main_worktree` returns `Result<bool>` instead of conflating error with false
    3. `WorktreeInfo` and `WorktreeStatus` have `deny_unknown_fields`, are registered in the schema registry, and `ahead`/`behind` use `u32` instead of `usize`
    4. Three missing tests added: `resolve_worktree_dir` with empty base_dir, `cleanup` with `force=true` on clean worktree, `parse_worktree_list` with malformed input
    5. `WorktreeDirty` error message is domain-only (no CLI-specific "run git stash" advice), `to_string_lossy` replaced with `OsStr`-aware handling, field duplication between `WorktreeInfo` and `WorktreeStatus` resolved

- [ ] Phase 55: Harness Crate & Profile Type
  - HARNESS-01: `assay-harness` crate as workspace leaf
  - HARNESS-02: `HarnessProfile` type in assay-types
  - **Goal:** Establish the harness crate boundary and the shared profile type — the data contract between core orchestration and adapter implementations
  - **Success criteria:**
    1. `assay-harness` crate exists in `crates/assay-harness/`, listed in workspace `Cargo.toml`, depends on assay-core and assay-types
    2. `HarnessProfile` in assay-types describes prompt template path, settings overrides, hook definitions, and target harness identifier — serializable with serde and schemars
    3. `assay-harness` builds and passes `just ready` with no warnings
    4. Dependency graph is verified: assay-harness depends on assay-core and assay-types; no reverse dependencies from core to harness

- [ ] Phase 56: Prompt Builder & Settings Merger
  - HARNESS-03: Layered prompt builder (project conventions + spec criteria)
  - HARNESS-04: Layered settings merger (project base + spec overrides)
  - **Goal:** Implement composable assembly of agent configuration — prompts and settings built from layers so project-wide conventions always apply and spec-specific overrides are additive
  - **Success criteria:**
    1. Prompt builder composes system prompt from project conventions layer (always included) and spec criteria layer (included when spec is provided)
    2. Settings merger applies project config base settings, then overlays spec-specific overrides for permissions, model, and tool access — later layers win on conflict
    3. Prompt builder with no spec provided produces a valid prompt containing only project conventions
    4. Settings merger with conflicting keys in base and spec layers resolves to spec layer value (last-write-wins)

- [ ] Phase 57: Hook Contract & Claude Code Adapter
  - HARNESS-05: Hook contract definitions for lifecycle events
  - HARNESS-06: Claude Code adapter generating CLAUDE.md, .mcp.json, settings, hooks.json
  - HARNESS-07: Callback-based control inversion for agent invocation
  - **Goal:** Define the hook lifecycle contract and implement the first concrete adapter — Claude Code — proving the callback-based control inversion pattern works end-to-end
  - **Success criteria:**
    1. Hook contract in assay-types declares `pre_tool`, `post_tool`, and `stop` lifecycle events as data types (not trait methods)
    2. Claude Code adapter generates CLAUDE.md content, `.mcp.json` configuration, settings overrides, and `hooks.json` from a `HarnessProfile` — all written to the worktree directory
    3. Agent invocation accepts closures for `on_launch`, `on_complete`, and `on_error` callbacks — no trait objects in the signature
    4. Generated CLAUDE.md includes spec criteria and project conventions assembled by the prompt builder from Phase 56
    5. Generated `.mcp.json` includes the Assay MCP server configuration pointing to the correct project root

- [ ] Phase 58: RunManifest
  - MANIFEST-01: `RunManifest` type with `[[sessions]]` TOML array
  - MANIFEST-02: Single-session manifest parsing with actionable errors
  - MANIFEST-03: Forward-compatible schema for multi-agent extension
  - **Goal:** Define the declarative work description format — a TOML manifest that describes what work to do, parsed and validated before any execution begins
  - **Success criteria:**
    1. `RunManifest` type in assay-types uses `sessions: Vec<SessionEntry>` field, corresponding to `[[sessions]]` TOML array format
    2. Single-session TOML manifest parses successfully — spec reference, harness identifier, and optional settings overrides are captured
    3. Malformed TOML input produces actionable error messages: line number, expected structure, and example of correct format
    4. Schema explicitly supports `Vec<SessionEntry>` (array, not single object) even though v0.5.0 only executes the first session — forward-compatible for v0.6.0 multi-agent

- [ ] Phase 59: End-to-End Pipeline
  - E2E-01: Single-agent pipeline (manifest → worktree → agent → gate → merge)
  - E2E-02: Pipeline exposed as MCP tool or composable tool sequence
  - E2E-03: Structured pipeline errors with stage and recovery guidance
  - **Goal:** Wire the full single-agent pipeline — the capstone that proves manifest-driven orchestration works end-to-end as an MCP-invocable operation
  - **Success criteria:**
    1. Single-agent pipeline executes: parse RunManifest → create worktree → configure harness → launch agent (via callback) → evaluate gates → propose merge — each stage observable
    2. Pipeline is invocable as an MCP tool (or composable sequence of MCP tools) that an outer agent can call to delegate work
    3. Pipeline failure at any stage produces a structured error identifying the failed stage name, the error detail, and recovery guidance (e.g., "worktree_create failed: spec already has active worktree — clean up with worktree_cleanup")
    4. Successful pipeline completion returns a structured result with worktree path, gate results summary, and merge proposal URL (or dry-run preview)

### ○ v0.6.0 Multi-Agent Orchestration

**Goal:** DAG executor, parallel sessions, `OrchestratorSession` composing `Vec<WorkSession>`, sequential merge, and `orchestrate_*` MCP tools.

### ○ v0.6.1 Conflict Resolution & Polish

**Goal:** AI conflict resolution via evaluator, Cupel integration for orchestrated sessions, Codex/OpenCode adapter stubs, `SessionCore` struct composition for type unification.

## Progress Summary

| Milestone | Status | Phases | Requirements | Complete |
|-----------|--------|--------|--------------|----------|
| v0.1.0 Proof of Concept | ✅ Shipped | 10 | 43 | 100% |
| v0.2.0 Dual-Track Gates & Hardening | ✅ Shipped | 15 | 52 | 100% |
| v0.3.0 Orchestration Foundation | ✅ Shipped | 9 | 43 | 100% |
| v0.4.0 Headless Orchestration | ✅ Shipped | 11 | 28 | 100% |
| v0.4.1 Merge Tools | ○ In Progress | 5 | 8 | 80% |
| v0.5.0 Single-Agent Harness E2E | ○ Planned | 9 | 19 | 0% |
| v0.6.0 Multi-Agent Orchestration | ○ Planned | TBD | TBD | — |
| v0.6.1 Conflict Resolution & Polish | ○ Planned | TBD | TBD | — |
