# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-02)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.2.0 Dual-Track Gates & Hardening

## Current Position

Phase: 11 — Type System Foundation (COMPLETE)
Plan: 02 of 2 (all complete)
Status: Complete
Last activity: 2026-03-04 — Completed 11-02-PLAN.md

Progress: v0.2.0 [#         ] ~8%

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 13 (11-23) | 52 | ~8% |

## Accumulated Context

### Decisions

v0.1.0 decisions archived to .planning/milestones/v0.1.0-ROADMAP.md

v0.2.0 decisions (from brainstorm + research):
- Agent gates receive evaluations via MCP, not call LLMs directly
- Self-evaluation + audit trail for v0.2; independent evaluator deferred to v0.3
- Keep core types domain-agnostic
- No built-in LLM client, no SpecProvider trait yet
- Pipeline semantics for future orchestrator design
- Type relocation (GateRunSummary -> assay-types) is highest-churn change — do first
- Agent-reported gates default to advisory enforcement (trust asymmetry)
- Per-spec subdirectory layout for results (.assay/results/{spec-name}/)
- Timestamp + 6-char random hex suffix for run IDs (no new crate)
- Include assay_version in GateRunRecord for future schema migration
- Two-tier enforcement only (required/advisory) — SonarQube validates no warnings tier
- Cozempic-inspired features (token diagnostics, team protection) added to v0.2.0 as phases 20-23
- Session JSONL parsing in Rust (not Python) — full feature parity with Cozempic, native performance
- Phases 20-23 are independent of 11-19 — can be worked in parallel or after gates
- Guard daemon uses kqueue (macOS) / inotify (Linux) for sub-second reactive recovery
- Pruning strategies compose sequentially, dry-run by default, team messages always protected

v0.2.0 decisions (from 11-01 execution):
- Clean break for type relocation: no re-exports from assay-core, all consumers import from assay_types
- Output types (GateRunSummary, CriterionResult) do NOT use deny_unknown_fields
- Schema registry entries added for both relocated types

v0.2.0 decisions (from 11-02 execution):
- Backward-compat test verifies GateRunSummary deserializes from minimal JSON without results field
- Skipped criterion test (result: None) verifies skip_serializing_if works correctly

### Pending Issues

38 open issues (expanded from 30 after Phase 8-10 PR reviews)

### Blockers

None.

### Next Actions

Phase 12: Gate Run Record — plan and execute (depends on Phase 11)

### Session Continuity

Last session: 2026-03-04
Stopped at: Completed Phase 11 (11-01 + 11-02 both done)
Resume file: None
