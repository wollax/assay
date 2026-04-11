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

<details>
<summary>✅ v0.4.1 Merge Tools — SHIPPED 2026-04-08</summary>

**Goal:** Ship merge conflict detection and PR-based merge proposal as MCP tools — enabling agents to safely check for conflicts and propose merges through pull requests with gate evidence, backed by forge-agnostic env vars and worktree fixes.

- [x] Phase 46: Worktree Fixes (3 plans, 3 waves — sequential, shared file) — 2026-03-16
- [x] Phase 47: Merge Check (2 plans, 2 waves — sequential) — 2026-03-16
- [x] Phase 48: Gate Evidence Formatting — 2026-03-16
- [x] Phase 49: Forge-Agnostic Env Vars — 2026-04-08
- [x] Phase 50: Merge Propose — 2026-04-08

[Full archive](milestones/v0.4.1-ROADMAP.md)

</details>

<details>
<summary>✅ v0.5.0 Single-Agent Harness End-to-End — SHIPPED 2026-04-08</summary>

**Goal:** Ship the complete single-agent pipeline — from declarative RunManifest through worktree isolation, harness-driven agent launch, gate evaluation, to merge proposal — proving Assay can orchestrate one agent end-to-end with callback-based control inversion and the Claude Code adapter.

**Note:** Phases 51-59 were implemented via Linear milestones M014-M024, which went beyond the original plan — adding streaming, multi-agent orchestration (DAG/mesh/gossip), checkpoint gates, auto-promote, Codex/OpenCode adapters, and Smelt monorepo integration. Phase 54 (worktree tech debt) partially complete — `WorktreeConfig.base_dir` intentionally kept as `String` to avoid schema-breaking change.

- [x] Phase 51: Session Vocabulary Cleanup — 2026-04-08
- [x] Phase 52: Session Persistence — 2026-04-08
- [x] Phase 53: Worktree Session Linkage — 2026-04-08
- [x] Phase 54: Worktree Tech Debt Batch (partial — base_dir type kept as String) — 2026-04-08
- [x] Phase 55: Harness Crate & Profile Type — 2026-04-08
- [x] Phase 56: Prompt Builder & Settings Merger — 2026-04-08
- [x] Phase 57: Hook Contract & Claude Code Adapter — 2026-04-08
- [x] Phase 58: RunManifest — 2026-04-08
- [x] Phase 59: End-to-End Pipeline — 2026-04-08

</details>

<details>
<summary>✅ v0.6.0 Multi-Agent Orchestration — SHIPPED 2026-04-08</summary>

**Goal:** DAG executor, parallel sessions, `OrchestratorSession` composing `Vec<WorkSession>`, sequential merge, and `orchestrate_*` MCP tools.

**Note:** Implemented via Linear milestones. Exceeded original scope with Mesh (SWIM heartbeat) and Gossip (coordinator broadcast) coordination modes in addition to DAG. 168 orchestration tests. `orchestrate_run` and `orchestrate_status` MCP tools shipped.

</details>

<details>
<summary>✅ v0.6.1 Conflict Resolution & Polish — SHIPPED 2026-04-08</summary>

**Goal:** AI conflict resolution via evaluator, Cupel integration for orchestrated sessions, Codex/OpenCode adapter stubs, `SessionCore` struct composition for type unification.

**Note:** AI conflict resolution (32.5K, Claude subprocess with validation command), Codex + OpenCode adapters (full implementations, not stubs), scope enforcement module shipped. `SessionCore` struct composition deferred — cosmetic refactor, not a feature gap.

</details>

<details>
<summary>✅ v0.6.2 P0 Cleanup — SHIPPED 2026-04-09</summary>

**Goal:** Resolve 27 P0 issues from post-M024 review findings — process safety, type correctness, serde consistency, and test coverage gaps.

- [x] Phase 60: Process Safety (5 requirements) — 2026-04-08
- [x] Phase 61: Type Correctness & Serde Consistency (7 requirements) — 2026-04-09
- [x] Phase 62: Review Findings (7 requirements) — 2026-04-09
- [x] Phase 63: Test Coverage Gaps (8 requirements) — 2026-04-09

Post-review fix: UTF-8 safe TextDelta truncation (floor_char_boundary) + correct OnEvent checkpoint phase metadata.

</details>

### v0.7.0 Gate Composability (In Progress)

**Milestone Goal:** Make gate definitions reusable, modular, and user-friendly — gate inheritance via `extends`, criteria libraries via `include`, spec preconditions, and a guided wizard across CLI, MCP, and TUI surfaces.

## Phases

- [ ] **Phase 64: Type Foundation** - Add composability types and backward-compat fields to `assay-types`
- [ ] **Phase 65: Resolution Core** - Criteria library I/O and `spec::compose::resolve()` with cycle detection
- [ ] **Phase 66: Evaluation Integration + Validation** - Wire resolution into gate evaluation, precondition enforcement, and `spec_validate` composability diagnostics
- [ ] **Phase 67: Wizard Core + CLI Surface** - Shared wizard logic in `assay-core` and interactive CLI commands
- [ ] **Phase 68: MCP Surface** - Five new MCP tools for agent-driven gate composition
- [ ] **Phase 69: TUI Surface** - TUI wizard state machine for human-facing gate editing

## Phase Details

### Phase 64: Type Foundation
**Goal**: The `assay-types` crate exposes all composability primitives — `CriteriaLibrary`, `SpecPreconditions`, `PreconditionStatus`, and three additive fields on `GatesSpec` — with schema snapshots updated and backward-compat roundtrip tests passing.
**Depends on**: Phase 63
**Requirements**: INHR-01, INHR-02, SAFE-03
**Success Criteria** (what must be TRUE):
  1. A gate TOML file with `extends = "parent-gate"` deserializes without error into `GatesSpec`
  2. A gate TOML file with `include = ["lib-name"]` deserializes without error into `GatesSpec`
  3. An existing pre-v0.7.0 TOML file (no composability fields) parses cleanly — no unknown-field errors, no missing defaults
  4. JSON schema snapshots include all new fields and compile without drift
**Plans:** 2 plans

Plans:
- [ ] 64-01-PLAN.md — New composability types (CriteriaLibrary, SpecPreconditions, PreconditionStatus) + GatesSpec fields + TDD tests
- [ ] 64-02-PLAN.md — Schema snapshot tests, roundtrip validation, workspace-wide verification

### Phase 65: Resolution Core
**Goal**: The `assay-core` crate can load, save, and scan criteria libraries from `.assay/criteria/`, and `spec::compose::resolve()` merges parent criteria into child gates with own-wins semantics, cycle detection, and per-criterion source tracking.
**Depends on**: Phase 64
**Requirements**: INHR-03, INHR-04, CLIB-01, CLIB-02, CLIB-03
**Success Criteria** (what must be TRUE):
  1. A criteria library TOML saved to `.assay/criteria/<slug>.toml` is loadable by name and appears in scan results
  2. Calling `resolve()` on a gate with `extends = "parent"` produces `effective_criteria` where parent criteria are present and own criteria override matching parent criteria by name
  3. A circular `extends` chain (A extends B extends A) causes `resolve()` to return a cycle-detection error rather than hang or panic
  4. Each criterion in resolved output carries a source annotation indicating whether it originated from the parent gate or the child gate
**Plans**: TBD

### Phase 66: Evaluation Integration + Validation
**Goal**: Gate evaluation runs resolved (flattened) criteria through the existing evaluator, precondition checks gate execution before criteria run, `PreconditionFailed` is a distinct non-failure result, and `spec_validate` reports composability errors.
**Depends on**: Phase 65
**Requirements**: PREC-01, PREC-02, PREC-03, SAFE-01, SAFE-02
**Success Criteria** (what must be TRUE):
  1. Running a gate on a spec whose `[preconditions].requires` references a spec with no passing gate run produces a `PreconditionFailed` result — gate criteria are not evaluated
  2. Running a gate whose `[preconditions].commands` contains a failing shell command produces a `PreconditionFailed` result distinct from a gate criterion failure
  3. `spec_validate` returns a structured diagnostic when `extends` references a non-existent parent gate
  4. `spec_validate` returns a structured diagnostic when `include` references a non-existent criteria library
  5. An `extends` or `include` value containing path traversal characters (e.g., `../evil`) is rejected by slug validation before any file I/O occurs
**Plans**: TBD

### Phase 67: Wizard Core + CLI Surface
**Goal**: `assay-core::wizard` exposes `apply_gate_wizard()` usable by any surface, and the CLI provides `assay gate wizard` (create/edit) and `assay criteria list/new` commands backed entirely by core validation logic.
**Depends on**: Phase 66
**Requirements**: WIZC-01, WIZC-02, WIZC-03
**Success Criteria** (what must be TRUE):
  1. `assay gate wizard` launches an interactive prompt flow that creates a new gate TOML file with user-supplied criteria, parent reference, and library includes
  2. `assay gate wizard --edit <gate>` loads an existing gate definition and allows the user to modify its criteria and composability fields, writing the result back
  3. `assay criteria list` displays all criteria libraries found in `.assay/criteria/` with their slug and criterion count
  4. `assay criteria new` creates a new criteria library file via an interactive prompt, rejecting invalid slugs before writing
**Plans**: TBD

### Phase 68: MCP Surface
**Goal**: Five new MCP tools expose agent-driven gate composition — `gate_wizard`, `criteria_list`, `criteria_get`, `criteria_create`, and `spec_resolve` — each delegating validation to `assay-core::wizard`.
**Depends on**: Phase 67
**Requirements**: WIZM-01, WIZM-02, WIZM-03, CLIB-04
**Success Criteria** (what must be TRUE):
  1. An agent calling `gate_wizard` with a gate name and criterion list receives a structured response and the gate TOML is written to disk
  2. An agent calling `criteria_list` receives a list of all available library slugs with criterion counts
  3. An agent calling `criteria_get` with a valid slug receives the full `CriteriaLibrary` payload; calling with an invalid slug returns a structured error
  4. An agent calling `criteria_create` with a slug and criteria list creates a new library file and returns the saved content
  5. An agent calling `spec_resolve` with a spec name receives the fully resolved `effective_criteria` list with source annotations for each criterion
**Plans**: TBD

### Phase 69: TUI Surface
**Goal**: The TUI provides a `GateWizardState`/`GateWizardAction` state machine with `handle_gate_wizard_event()` and `draw_gate_wizard()`, delegating all field validation to `assay-core::wizard` with no surface-specific logic.
**Depends on**: Phase 67
**Requirements**: WIZT-01, WIZT-02
**Success Criteria** (what must be TRUE):
  1. From the TUI, the user can navigate to the gate wizard screen, fill in gate fields (name, criteria, parent, libraries), and confirm to write the gate TOML to disk
  2. From the TUI, the user can select an existing gate and edit its composability fields, with the result written back to disk
  3. Invalid inputs (bad slug, missing name) are rejected by core validation before the wizard attempts any file write — no validation logic lives in TUI code
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order. Phases 68 and 69 are independent and can execute in parallel after Phase 67 completes.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 64. Type Foundation | 0/2 | Planning complete | - |
| 65. Resolution Core | 0/TBD | Not started | - |
| 66. Evaluation Integration + Validation | 0/TBD | Not started | - |
| 67. Wizard Core + CLI Surface | 0/TBD | Not started | - |
| 68. MCP Surface | 0/TBD | Not started | - |
| 69. TUI Surface | 0/TBD | Not started | - |

## Progress Summary

| Milestone | Status | Phases | Requirements | Complete |
|-----------|--------|--------|--------------|----------|
| v0.1.0 Proof of Concept | ✅ Shipped | 10 | 43 | 100% |
| v0.2.0 Dual-Track Gates & Hardening | ✅ Shipped | 15 | 52 | 100% |
| v0.3.0 Orchestration Foundation | ✅ Shipped | 9 | 43 | 100% |
| v0.4.0 Headless Orchestration | ✅ Shipped | 11 | 28 | 100% |
| v0.4.1 Merge Tools | ✅ Shipped | 5 | 8 | 100% |
| v0.5.0 Single-Agent Harness E2E | ✅ Shipped | 9 | 19 | 100% |
| v0.6.0 Multi-Agent Orchestration | ✅ Shipped | — | — | 100% |
| v0.6.1 Conflict Resolution & Polish | ✅ Shipped | — | — | 100% |
| v0.6.2 P0 Cleanup | ✅ Shipped | 4 | 27 | 100% |
| v0.7.0 Gate Composability | 🚧 In progress | 6 | 22 | 0% |
