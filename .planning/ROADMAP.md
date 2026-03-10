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

#### Phase 29: Gate Output Truncation

**Goal:** Implement head+tail output capture with byte budgets so gate command output is bounded, UTF-8 safe, and truncation is visible in results.
**Dependencies:** Phase 27 (GateResult type refinements)
**Requirements:** GATE-01, GATE-02, GATE-03, GATE-04, GATE-05
**Success Criteria** (what must be TRUE):
  1. Gate command output exceeding the byte budget is truncated with head and tail sections preserved
  2. Truncated output contains a `[truncated: X bytes omitted]` marker between head and tail
  3. Truncation never splits a multi-byte UTF-8 sequence (verified by test with multi-byte input)
  4. stdout and stderr have independent byte budgets (one can truncate while the other doesn't)
  5. `GateResult.truncated` is `true` and `GateResult.original_bytes` reflects the pre-truncation size when truncation occurs

#### Phase 30: Core Tech Debt

**Goal:** Eliminate validation duplication, extract shared evaluation logic, harden history and daemon persistence, and tighten visibility on internal APIs.
**Dependencies:** Phase 26 (error ergonomics)
**Requirements:** CORE-02, CORE-03, CORE-04, CORE-06, CORE-07, CORE-08, CORE-09
**Success Criteria** (what must be TRUE):
  1. `validate()` and `validate_gates_spec()` share a single validation implementation (no duplicated enforcement logic)
  2. `evaluate_all` and `evaluate_all_gates` use a shared extraction (no duplicated iteration/collection logic)
  3. `history::list()` emits a warning for unreadable directory entries instead of silently dropping them
  4. `generate_run_id` is `pub(crate)` (not `pub`)
  5. Guard daemon PID file write is followed by `fsync()` and `try_save_checkpoint` uses stored project dir

#### Phase 31: Error Messages

**Goal:** Make all error messages actionable — command-not-found errors name the missing binary, spec-not-found errors list available specs, and TOML parse errors include file path and line number.
**Dependencies:** Phase 26 (error ergonomics), Phase 30 (spec parse error handling)
**Requirements:** ERR-01, ERR-02, ERR-03
**Success Criteria** (what must be TRUE):
  1. A gate run with a nonexistent command shows "Command 'X' not found. Is it installed and in PATH?"
  2. Requesting a nonexistent spec shows the spec name and lists all available spec names
  3. An invalid TOML spec file shows the file path, line number, and specific parse error message

#### Phase 32: CLI Polish

**Goal:** Fix correctness issues and eliminate code duplication across the CLI surface — NO_COLOR handling, help text, enforcement blocks, color branches, StreamCounters, and magic strings.
**Dependencies:** Phase 26 (CLI modules extracted)
**Requirements:** CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06, CLI-07, CLI-08
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
| v0.3.0 Orchestration Foundation | 🔄 In Progress | 8 (26-33) | 43 | 38% |
