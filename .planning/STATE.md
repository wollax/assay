# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-15)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.5.0 Single-Agent Harness End-to-End — Defining requirements

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-03-15 — Milestone v0.5.0 started

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 15 (11-25) | 52 | 100% (shipped) |
| v0.3.0 | 9 (26-34) | 43 | 100% (shipped) |
| v0.4.0 | 11 (35-45) | 28 | 100% (shipped) |
| v0.4.1 | 5 (46-50) | 8 | 0% (planned) |
| v0.5.0 | TBD | TBD | 0% (defining) |

## Accumulated Context

### Decisions

v0.1.0 decisions archived to .planning/milestones/v0.1.0-ROADMAP.md
v0.2.0 decisions archived to .planning/milestones/v0.2.0-ROADMAP.md
v0.3.0 decisions archived to .planning/milestones/v0.3.0-ROADMAP.md
v0.4.0 decisions archived to .planning/milestones/v0.4.0-ROADMAP.md

v0.4.1 decisions (from brainstorm):
- PR creation over direct merge for v0.4.x — maps to `autonomous: false`
- `git merge-tree --write-tree` for conflict detection — zero side effects
- GitHub-first via `gh` CLI, env vars for forge-agnostic extensibility
- Hardcode merge defaults, extract config from usage (YAGNI)
- Auto-revert killed permanently — contradicts `autonomous: false`
- Investigate GitHub merge queue before building multi-worktree ordering

v0.5.0 decisions (from brainstorm 2026-03-15T16-14):
- Absorb Smelt orchestration into Assay; Smelt pivots to infrastructure-only
- Closures for control inversion, not traits (zero-trait codebase convention)
- Orchestration as `assay-core::orchestrate` module, not separate crate
- `assay-harness` as new leaf crate for adapter implementations
- `OrchestratorSession` composes `Vec<WorkSession>` (v0.6.0)
- Additive `orchestrate_*` MCP tools, don't modify existing tools (v0.6.0)
- Worktrees stay spec-scoped; session linkage is additive
- `[[sessions]]` array in RunManifest from day one (forward-compatible)
- Session vocabulary cleanup: `AgentSession` → `GateEvalContext`
- Struct composition (`SessionCore`) over traits for type unification (v0.6.1)

### Milestone Scope Issues

Issues pulled into v0.4.1 scope:
- "Default branch fallback to main gives confusing errors" (from: .planning/issues/open/2026-03-09-worktree-detect-default-branch-fallback.md)
- "Git worktree prune failure silently discarded" (from: .planning/issues/open/2026-03-09-worktree-prune-failure-silent.md)

### Pending Issues

124 open issues in .planning/issues/open/ (non-blocking tech debt carried from v0.2.0–v0.4.0)
See .planning/issues/ for full backlog.

### Blockers

None. v0.4.1 must ship before v0.5.0 work begins.

### Next Actions

Define v0.5.0 requirements, then create roadmap.

### Session Continuity

Last session: 2026-03-15
Stopped at: v0.5.0 milestone definition in progress
Resume file: None
