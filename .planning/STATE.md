# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-02)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.2.0 Dual-Track Gates & Hardening

## Current Position

Phase: 11 — Type System Foundation
Plan: —
Status: Roadmap created, ready for planning
Last activity: 2026-03-02 — Roadmap created (phases 11-19)

Progress: v0.2.0 [          ] 0%

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 9 (11-19) | 31 | 0% |

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

### Pending Issues

38 open issues (expanded from 30 after Phase 8-10 PR reviews)

### Blockers

None.

### Next Actions

Phase 11: Type System Foundation — create plan(s) and implement

### Session Continuity

Last session: 2026-03-02
Stopped at: Roadmap created, ready for Phase 11 planning
Resume file: None
