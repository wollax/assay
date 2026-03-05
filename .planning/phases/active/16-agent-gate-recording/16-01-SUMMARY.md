---
phase: "16"
plan: "01"
title: "Agent Gate Types and Criterion Extension"
subsystem: "types"
tags: ["agent-gates", "session", "evaluation", "serde", "schema"]
dependency_graph:
  requires: ["11-01", "12-01", "13-01"]
  provides: ["GateKind::AgentReport", "CriterionKind", "EvaluatorRole", "Confidence", "AgentEvaluation", "AgentSession"]
  affects: ["16-02", "16-03", "16-04"]
tech_stack:
  added: []
  patterns: ["optional agent fields on output types", "CriterionKind enum for dispatch routing"]
key_files:
  created:
    - crates/assay-types/src/session.rs
  modified:
    - crates/assay-types/src/gate.rs
    - crates/assay-types/src/criterion.rs
    - crates/assay-types/src/gates_spec.rs
    - crates/assay-types/src/lib.rs
    - crates/assay-types/tests/schema_roundtrip.rs
    - crates/assay-types/tests/snapshots/schema_snapshots__criterion-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-criterion-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gates-spec-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-kind-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-result-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-run-summary-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__spec-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__workflow-schema.snap
decisions:
  - "CriterionKind enum is a simple enum (not internally tagged) — single variant AgentReport for now"
  - "Agent fields on GateResult are all Option<T> with skip_serializing_if for backward compat"
  - "AgentSession uses HashMap/HashSet for flexible keying — not Vec — to support O(1) criterion lookup"
  - "EvaluatorRole::SelfEval serializes as 'self' via serde rename (kebab-case + rename on variant)"
  - "Downstream crate compile errors (assay-core, assay-mcp) are expected and deferred to Plan 02"
metrics:
  duration: "~20 minutes"
  completed: "2026-03-05"
---

# Phase 16 Plan 01: Agent Gate Types and Criterion Extension Summary

**One-liner:** Session/evaluation types (EvaluatorRole, Confidence, AgentEvaluation, AgentSession), GateKind::AgentReport variant, CriterionKind enum, and kind/prompt fields on Criterion/GateCriterion — all backward-compatible via serde defaults.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Create session types module | e2c85c8 | session.rs, lib.rs |
| 2 | Extend GateKind and GateResult with agent support | 4265f48 | gate.rs, schema_roundtrip.rs, 4 snapshots |
| 3 | Add kind/prompt to Criterion and GateCriterion | c2edae9 | criterion.rs, gates_spec.rs, lib.rs, schema_roundtrip.rs, 5 snapshots |

## What Was Built

### Task 1: Session Types Module (`session.rs`)
- **EvaluatorRole** enum: `SelfEval` (serializes as `"self"`), `Independent`, `Human`
- **Confidence** enum: `High`, `Medium`, `Low` (all kebab-case)
- **AgentEvaluation** struct: passed, evidence, reasoning, confidence (optional), evaluator_role, timestamp
- **AgentSession** struct: session_id, spec_name, created_at, command_results, agent_evaluations (HashMap), criteria_names (HashSet), spec_enforcement (HashMap)
- All four types registered in schema registry with `inventory::submit!`
- 10 tests covering serialization, roundtrip, and field omission

### Task 2: GateKind::AgentReport and GateResult Agent Fields
- Added `AgentReport` unit variant to `GateKind` (serializes as `kind = "AgentReport"`)
- Added 4 optional fields to `GateResult`: evidence, reasoning, confidence, evaluator_role
- Updated 8 existing test constructions + 3 integration test files
- Updated 4 insta snapshots (gate-kind, gate-result, criterion-result, gate-run-summary)
- 4 new tests: AgentReport TOML roundtrip, field skip/include, full roundtrip

### Task 3: CriterionKind and kind/prompt Fields
- Created `CriterionKind` enum with `AgentReport` variant + schema registry entry
- Added `kind: Option<CriterionKind>` and `prompt: Option<String>` to both `Criterion` and `GateCriterion`
- Re-exported `CriterionKind` from `assay-types` lib.rs
- Updated all existing constructions in unit tests and integration tests
- Updated 5 insta snapshots
- 4 new tests: agent report roundtrip for both types, kind omission, mixed spec roundtrip

## Deviations from Plan

None — plan executed exactly as written.

## Known Downstream Impact

As expected by the plan, `assay-core` and `assay-mcp` do not compile after this change:
- 7 `GateResult` constructions in `assay-core/src/gate/mod.rs` missing new agent fields
- 1 `Criterion` construction in `assay-core/src/gate/mod.rs` missing `kind`/`prompt`
- `assay-mcp/src/server.rs` has similar missing fields

These are mechanical fixes (add `None` for all new optional fields) and are planned for 16-02.

## Test Results

- **assay-types unit tests:** 85 passed, 0 failed
- **assay-types clippy:** clean (0 warnings)

## Next Phase Readiness

Plan 02 can proceed immediately. All types needed for agent gate evaluation dispatch are now in place:
- `CriterionKind::AgentReport` maps to `GateKind::AgentReport` in dispatch logic
- `AgentSession` provides crash-recovery state for in-progress evaluations
- `AgentEvaluation` structures agent reasoning for audit trails
