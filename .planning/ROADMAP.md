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

### 🔄 v0.4.0 Headless Orchestration

**Goal:** Ship `gate_evaluate` as the capstone MCP tool — agent-evaluated gates driven by headless subprocess orchestration, backed by session persistence, context-aware diff budgeting, spec validation, and observability improvements.

- [x] Phase 35: Observability Foundation — 2026-03-11
  - OBS-01: `warnings` field on mutating MCP responses
  - OBS-02: Outcome-filtered `gate_history` with limit
  - DEBT-02: Close history-save-failure-not-surfaced issue

- [x] Phase 36: Correctness & Robustness — 2026-03-11
  - FIX-01: Worktree status relative to base branch
  - FIX-02: Better gate_report/gate_finalize error messages
  - FIX-03: Diff context attached to gate sessions

- [x] Phase 37: Spec Validation — 2026-03-11
  - SPEC-01: `spec_validate` MCP tool with structured diagnostics
  - SPEC-02: TOML parse, criterion uniqueness, prompt field, structure completeness
  - SPEC-03: Optional `check_commands` parameter for PATH validation
  - SPEC-04: Cross-spec dependency validation with cycle detection

- [x] Phase 38: Observability Completion — 2026-03-13
  - OBS-03: `spec_get` resolved config with timeout precedence
  - OBS-04: Growth rate metrics in `estimate_tokens`

- [x] Phase 39: Context Engine Integration — 2026-03-15
  - CTX-01: External context-engine crate dependency
  - CTX-02: Integration surface definition
  - CTX-03: Fallback behavior without context engine
  - **Success criteria:**
    1. `context-engine` crate is added as a dependency and compiles with assay workspace
    2. Integration surface is defined — context engine provides budget allocation, assay provides content sources (diff, spec, criteria)
    3. When context engine is unavailable or budget exceeds content size, content passes through without truncation
    4. Token budget calculation accepts model window size and deducts spec criteria and system prompt sizes

- [x] Phase 40: WorkSession Type & Persistence — 2026-03-15
  - SESS-01: WorkSession type persisted as JSON
  - SESS-02: Phase transitions with timestamps
  - **Success criteria:**
    1. `WorkSession` is persisted as JSON under `.assay/sessions/<session-id>.json` linking worktree path, spec name, agent invocation, and gate run references
    2. Phase transitions (`created → agent_running → gate_evaluated → completed | abandoned`) are tracked with timestamps
    3. Sessions are loadable from disk and round-trip through JSON serialization without data loss

- [x] Phase 41: Session MCP Tools — 2026-03-15
  - SESS-03: `session_create` MCP tool
  - SESS-04: `session_update` MCP tool
  - SESS-05: `session_list` MCP tool
  - **Success criteria:**
    1. `session_create` creates and persists a new session, returning session ID and initial state
    2. `session_update` transitions session phase and links gate run IDs — invalid transitions are rejected with clear errors
    3. `session_list` enumerates sessions with optional `spec_name` and `status` filters returning matching sessions
    4. All session MCP tools include `warnings` field on responses (from Phase 35)

- [x] Phase 42: Session Recovery & Internal API — 2026-03-15
  - SESS-06: Startup recovery for stale sessions
  - SESS-07: gate_evaluate uses Rust functions, not MCP round-trips
  - **Success criteria:**
    1. On startup, `.assay/sessions/` is scanned for stale `agent_running` sessions — each is marked `abandoned` with a recovery note and timestamp
    2. `gate_evaluate` calls session management through direct Rust function calls, never MCP round-trips
    3. Recovery scan handles corrupt session files gracefully — logs warning, skips file, continues scan

- [ ] Phase 43: gate_evaluate Schema & Subprocess (2 plans)
  - ORCH-01: gate_evaluate evaluates agent criteria in single call
  - ORCH-02: Subprocess model — parent parses JSON, evaluator never calls MCP
  - ORCH-03: EvaluatorOutput JSON schema with lenient parsing
  - Plan 01 (wave 1): EvaluatorOutput types + GatesConfig extension + core evaluator module (subprocess, prompt, parsing, mapping)
  - Plan 02 (wave 2): gate_evaluate MCP tool handler with 10-step flow + session auto-linking
  - **Success criteria:**
    1. `gate_evaluate` MCP tool computes diff, spawns headless Claude Code evaluator (`--print --output-format json`), parses per-criterion results, and persists GateRunRecord
    2. Evaluator subprocess never calls MCP tools — parent process owns all parsing and persistence
    3. `EvaluatorOutput` JSON schema is defined before prompt engineering — lenient `serde_json::Value` intermediate parse handles unexpected fields gracefully
    4. Per-criterion results include pass/fail status and evaluator reasoning

- [ ] Phase 44: gate_evaluate Context Budgeting
  - ORCH-04: Diff token budget via context engine
  - ORCH-05: Head-first + tail fallback truncation
  - **Success criteria:**
    1. `gate_evaluate` computes diff token budget as: model context window − spec criteria tokens − system prompt tokens
    2. Diff is truncated to budget using head-first strategy with tail fallback — preserving most important context
    3. Truncation metadata (original size, truncated size, strategy used) is included in GateRunRecord
    4. When diff fits within budget, no truncation occurs and no truncation metadata is recorded

- [ ] Phase 45: Tech Debt Cleanup
  - DEBT-01: Batch sweep of highest-value backlog issues
  - **Success criteria:**
    1. Backlog issues that interact with v0.4.0 changes (worktree, MCP, types, history) are prioritized and resolved
    2. At least 10 open issues from `.planning/issues/open/` are closed
    3. All resolved issues are verified by `just ready` passing

### ○ v0.4.1 Merge Tools

**Goal:** Ship merge conflict detection and PR-based merge proposal as MCP tools — enabling agents to safely check for conflicts and propose merges through pull requests with gate evidence, backed by forge-agnostic env vars and worktree fixes.

- [ ] Phase 46: Worktree Fixes
  - WFIX-01: Cleanup `--all` uses canonical path from git
  - WFIX-02: Default branch detection provides actionable error
  - WFIX-03: Prune failures surfaced as warnings
  - **Goal:** Fix worktree edge cases before building merge tools on top of worktree infrastructure
  - **Success criteria:**
    1. `worktree cleanup --all` resolves paths via `git worktree list` canonical output instead of string comparison — handles symlinks and relative paths
    2. Default branch detection fails with actionable error message naming the `init.defaultBranch` config key instead of silently falling back to `main`
    3. `git worktree prune` failures are surfaced as warnings in MCP responses (via Phase 35 `warnings` field) instead of silently discarded

- [ ] Phase 47: Merge Check
  - MERGE-01: `merge_check` MCP tool
  - **Goal:** Standalone conflict detection via `git merge-tree --write-tree` with zero side effects
  - **Success criteria:**
    1. `merge_check` MCP tool accepts `base` and `head` refs and returns `MergeCheck { clean, conflicts, base_sha, head_sha }`
    2. Uses `git merge-tree --write-tree` (Git 2.38+) — no index mutation, no working tree changes
    3. When merge is clean, `conflicts` is empty and `clean` is true
    4. When merge has conflicts, `conflicts` lists affected file paths and `clean` is false

- [ ] Phase 48: Gate Evidence Formatting
  - MERGE-04: Gate evidence formatting for PR body
  - **Goal:** Format gate results as markdown suitable for PR bodies with GitHub character limit handling
  - **Success criteria:**
    1. Gate results are formatted as markdown with per-criterion pass/fail status and evaluator reasoning
    2. PR body is truncated at 65,536 chars with a link to the full gate report path
    3. Truncation preserves the summary section and truncates individual criterion details
    4. When gate results fit within limit, full content is included without truncation markers

- [ ] Phase 49: Forge-Agnostic Env Vars
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

## Progress Summary

| Milestone | Status | Phases | Requirements | Complete |
|-----------|--------|--------|--------------|----------|
| v0.1.0 Proof of Concept | ✅ Shipped | 10 | 43 | 100% |
| v0.2.0 Dual-Track Gates & Hardening | ✅ Shipped | 15 | 52 | 100% |
| v0.3.0 Orchestration Foundation | ✅ Shipped | 9 | 43 | 100% |
| v0.4.0 Headless Orchestration | 🔄 In Progress | 11 | 28 | 64% |
| v0.4.1 Merge Tools | ○ Planned | 5 | 8 | 0% |
