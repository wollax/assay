# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-02)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.2.0 Dual-Track Gates & Hardening

## Current Position

Phase: 13 — Enforcement Levels (VERIFIED & COMPLETE)
Plan: 03 of 3 (complete)
Status: Phase complete, verified, moved to completed/
Last activity: 2026-03-04 — Phase 13 verified and completed

Progress: v0.2.0 [███       ] ~23%

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 13 (11-23) | 52 | ~15% |

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

v0.2.0 decisions (from 12-01 execution):
- cmd takes precedence over path when both set (simpler than mutual exclusivity validation)
- path field uses same serde attributes as cmd (skip_serializing_if + default)
- evaluate_file_exists implementation unchanged — already correct

v0.2.0 decisions (from 13-01 execution):
- Enforcement enum uses Copy trait (two-variant fieldless, read frequently during evaluation)
- Input types use Option<Enforcement> (None = inherit from gate section default); output types use concrete Enforcement
- GateSection uses deny_unknown_fields (user-authored input); EnforcementSummary does not (output type)

v0.2.0 decisions (from 13-02 execution):
- Backward compat preserved: passed/failed/skipped counts compute as before; EnforcementSummary is additive
- resolve_enforcement() is public for reuse by CLI/MCP pass/fail logic
- Validation enforces at-least-one-required at parse time, not evaluation time
- Descriptive-only criteria (no cmd/path) do not count as executable for the required check

### Pending Issues

38 open issues (expanded from 30 after Phase 8-10 PR reviews)

### Blockers

None.

### Next Actions

Phase 13 verified and complete. Next: Phase 14 — Run History Core

### Session Continuity

Last session: 2026-03-04
Stopped at: Phase 13 verified and completed, PR ready for review
Resume file: None
