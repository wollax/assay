# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-10)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.4.0 Headless Orchestration — gate_evaluate capstone, session persistence, context engine integration

## Current Position

Phase: 36 — Correctness & Robustness
Plan: 2 of 3 (Enriched Gate Session Error Messages)
Status: Complete
Last activity: 2026-03-11 — Completed 36-02-PLAN.md

Progress: v0.4.0 [██░░░░░░░░░░░░░░░] ~9% (phase 35 complete, 10 phases remaining)

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 15 (11-25) | 52 | 100% (shipped) |
| v0.3.0 | 9 (26-34) | 43 | 100% (shipped) |
| v0.4.0 | 11 (35-45) | 28 | 9% (1/11 phases) |
| v0.4.1 | TBD | 8 | 0% (planned) |

## Accumulated Context

### Decisions

v0.1.0 decisions archived to .planning/milestones/v0.1.0-ROADMAP.md
v0.2.0 decisions archived to .planning/milestones/v0.2.0-ROADMAP.md
v0.3.0 decisions archived to .planning/milestones/v0.3.0-ROADMAP.md

v0.4.0 decisions (from brainstorm):
- `gate_evaluate` uses subprocess model — parent parses structured JSON output, evaluator never calls MCP tools
- `EvaluatorOutput` schema defined before prompt engineering — lenient `serde_json::Value` intermediate parse
- `WorkSession` (on-disk) is distinct from `AgentSession` (in-memory v0.3.0)
- Session management within `gate_evaluate` is Rust function calls, not MCP round-trips
- Context engine is external crate (separate repo), not workspace crate
- `spec_validate` check_commands is opt-in (off by default)

v0.4.0 decisions (from 35-01):
- `build_finalized_record` returns plain `GateRunRecord` (infallible without I/O)
- `persisted` field on `GateFinalizeResponse` derives from `warnings.is_empty()`
- `finalize_session` kept as backward-compat wrapper

v0.4.0 decisions (from 35-02):
- Unrecognized outcome values treated as "any" (graceful degradation)
- `total_runs` reflects on-disk count, not filtered count

v0.4.0 decisions (from 36-02):
- `timed_out_sessions` capped at 100 entries with oldest-eviction
- Session not-found errors always suggest gate_run + gate_history (no active session listing)
- `create_session` diff params wired as None/false/None until diff capture is implemented

v0.4.1 decisions (from brainstorm):
- PR creation over direct merge for v0.4.x — maps to `autonomous: false`
- `git merge-tree --write-tree` for conflict detection — zero side effects
- GitHub-first via `gh` CLI, env vars for forge-agnostic extensibility
- Hardcode merge defaults, extract config from usage (YAGNI)
- Auto-revert killed permanently — contradicts `autonomous: false`
- Investigate GitHub merge queue before building multi-worktree ordering

### Milestone Scope Issues

Issues pulled into v0.4.0 scope:
- "History save failure not surfaced" (from: .planning/issues/open/2026-03-10-history-save-failure-not-surfaced.md) — closed by OBS-01 warnings field
- Tech debt batch sweep of highest-value backlog items

Issues pulled into v0.4.1 scope:
- "Cleanup --all should use canonical path from git" (from: .planning/issues/open/2026-03-09-worktree-cleanup-all-path.md)
- "Default branch fallback to main gives confusing errors" (from: .planning/issues/open/2026-03-09-worktree-detect-default-branch-fallback.md)
- "Git worktree prune failure silently discarded" (from: .planning/issues/open/2026-03-09-worktree-prune-failure-silent.md)

### Pending Issues

100+ open issues in .planning/issues/open/ (test gaps, derives, naming, refactors)
See .planning/issues/ for full backlog.

### Blockers

None.

### Next Actions

Run `/kata-plan-phase [N]` to start planning phases, or `/kata-discuss-phase [N]` to gather context first.

### Session Continuity

Last session: 2026-03-11
Stopped at: Completed 36-02-PLAN.md
Resume file: None
