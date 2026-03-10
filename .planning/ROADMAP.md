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

### 🔄 v0.3.0 Orchestration Foundation (In Progress)

**Goal:** Build the foundation for agent orchestration — worktree isolation, independent gate evaluation infrastructure, and CLI/MCP/types/core hardening — while closing tech debt from v0.2.0.

- [x] Phase 26: Structural Prerequisites (2 plans) — 2026-03-09
- [x] Phase 27: Types Hygiene (4 plans) — 2026-03-09
- [x] Phase 28: Worktree Manager (2 plans) — 2026-03-09
- [x] Phase 29: Gate Output Truncation (2 plans) — 2026-03-09
- [x] Phase 30: Core Tech Debt (3 plans) — 2026-03-10
- [x] Phase 31: Error Messages (2 plans) — 2026-03-10

#### Phase 32: CLI Polish (4 plans)

**Goal:** Fix correctness issues and eliminate code duplication across the CLI surface — NO_COLOR handling, help text, enforcement blocks, color branches, StreamCounters, and magic strings.
**Dependencies:** Phase 26 (CLI modules extracted)
**Requirements:** CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06, CLI-07, CLI-08
**Plans:**
  - Plan 01 (Wave 1): CLI-08 constant + CLI-01 NO_COLOR fix + CLI-07 column gap — shared modules
  - Plan 02 (Wave 1): CLI-05 StreamCounters methods + CLI-06 StreamConfig docs — gate.rs structs
  - Plan 03 (Wave 1): CLI-02 help text dedup + CLI-04 color branch dedup — main.rs + spec.rs cleanup
  - Plan 04 (Wave 2, depends on 02): CLI-03 enforcement dedup — uses gate_blocked() from Plan 02
**Success Criteria** (what must be TRUE):
  1. Setting `NO_COLOR=1` disables all color output; unsetting it enables color (using `var_os().is_none()`)
  2. Gate command help text appears once (no duplication between top-level and subcommand)
  3. Enforcement check logic exists in one place (shared between `handle_gate_run_all` and `handle_gate_run`)
  4. `StreamCounters` has doc comments, a `tally()` method, and a `gate_blocked()` method
  5. The `[srs]` magic string is extracted to a named constant

#### Phase 33: MCP Validation

**Goal:** Harden MCP tool parameter validation with specific error messages, improve spec-not-found diagnostics, check stdout for failure reasons, and remove unnecessary clones.
**Dependencies:** Phase 31 (error message patterns established)
**Requirements:** MCP-01, MCP-02, MCP-03, MCP-04, MCP-05
**Success Criteria** (what must be TRUE):
  1. Calling an MCP tool with a missing required parameter returns a specific error naming the parameter
  2. Calling an MCP tool with an invalid parameter type returns a specific error naming the parameter and expected type
  3. A spec-not-found MCP error includes the list of available spec names
  4. MCP gate failure reason checks stdout in addition to stderr
  5. `gate_run` handler has no unnecessary clone intermediaries

## Progress Summary

| Milestone | Status | Phases | Requirements | Complete |
|-----------|--------|--------|--------------|----------|
| v0.1.0 Proof of Concept | ✅ Shipped | 10 | 43 | 100% |
| v0.2.0 Dual-Track Gates & Hardening | ✅ Shipped | 15 | 52 | 100% |
| v0.3.0 Orchestration Foundation | 🔄 In Progress | 8 (26-33) | 43 | 75% |
