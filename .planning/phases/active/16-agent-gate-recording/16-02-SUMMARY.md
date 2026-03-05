---
phase: "16"
plan: "02"
title: "Core Evaluation Dispatch and Validation for Agent Criteria"
status: complete
started: "2026-03-05T20:34:26Z"
completed: "2026-03-05T20:49:00Z"
duration: "~15 min"
tasks_completed: 2
tasks_total: 2
---

# 16-02 Summary: Core Evaluation Dispatch and Validation for Agent Criteria

All downstream compile errors from Plan 01 are fixed, AgentReport dispatch is wired into evaluation, session lifecycle is functional, and spec validation enforces mutual exclusivity constraints.

## Task Results

### Task 1: Fix downstream compile errors and add AgentReport dispatch
**Commit:** `9b52dd5`

- Added `evidence`, `reasoning`, `confidence`, `evaluator_role` (all `None`) to every `GateResult` constructor in assay-core (gate/mod.rs, history/mod.rs) and assay-mcp (server.rs)
- Added `kind: None, prompt: None` to all `Criterion` and `GateCriterion` test literals across gate/mod.rs, spec/mod.rs, history/mod.rs
- Updated `to_criterion()` to forward `kind` and `prompt` from `GateCriterion`
- Updated `gate_kind_for()` to return `GateKind::AgentReport` for agent criteria
- `evaluate()` returns `AssayError::InvalidCriterion` for AgentReport criteria (cannot be evaluated standalone)
- `evaluate_all()` and `evaluate_all_gates()` skip AgentReport criteria (result: None, increment skipped)
- Added error variants: `SessionNotFound`, `InvalidCriterion`, `SessionError`
- Created `pub mod session` declaration in gate/mod.rs
- Tests: `evaluate_agent_criterion_returns_error`, `evaluate_all_with_agent_criterion_marks_as_skipped`

### Task 2: Session lifecycle and spec validation
**Commit:** `8cc17e8`

- Implemented `crates/assay-core/src/gate/session.rs` with:
  - `create_session()` — generates session with unique ID via `history::generate_run_id`
  - `report_evaluation()` — validates criterion name, appends to evaluation map
  - `finalize_session()` — combines command + agent results, resolves evaluator priority, saves via `history::save()`
  - `finalize_as_timed_out()` — marks unevaluated required criteria as failed, does NOT save
  - `resolve_evaluator_priority()` — Human > Independent > SelfEval
- Added AgentReport mutual exclusivity validation to both `validate()` and `validate_gates_spec()`:
  - Rejects `kind=AgentReport` with `cmd` (error)
  - Rejects `kind=AgentReport` with `path` (error)
- Updated `is_executable` check: AgentReport criteria now count as executable for the at-least-one-required validation
- Tests: 9 new (7 session lifecycle, 2 validation)

## Deviations

1. **History test GateResult constructors** — Plan only mentioned assay-cli and assay-mcp as downstream sites, but history/mod.rs tests also construct `GateResult` literals. Fixed as auto-fix (Rule 1).

2. **AgentReport criteria counted as executable** — The plan didn't explicitly call out updating the "at least one required executable criterion" check in spec validation. Added this because without it, a spec with only agent criteria would fail validation with "must have a cmd or path field". Auto-fix (Rule 2).

## Decisions

- `finalize_as_timed_out()` does NOT call `history::save()` — caller decides whether to persist timeout records
- Unevaluated advisory criteria in timed-out sessions are skipped (not failed)
- AgentReport criteria count as "executable" for the at-least-one-required validation check

## Verification

- `cargo test --workspace`: 245 passed, 3 ignored
- `cargo clippy --workspace -- -D warnings`: clean
