# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-10)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.4.0 Headless Orchestration — gate_evaluate capstone, session persistence, context engine integration

## Current Position

Phase: 40 — WorkSession Type & Persistence
Plan: 1 of 2 (WorkSession data model)
Status: In progress
Last activity: 2026-03-15 — Completed 40-01-PLAN.md

Progress: v0.4.0 [██████████░░░░░░░] ~50% (phases 35-39 complete, 40 in progress)

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 15 (11-25) | 52 | 100% (shipped) |
| v0.3.0 | 9 (26-34) | 43 | 100% (shipped) |
| v0.4.0 | 11 (35-45) | 28 | 45% (5/11 phases) |
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

v0.4.0 decisions (from 36-03):
- Diff capture uses `std::process::Command` directly (not assay_core worktree module) to avoid error type coupling
- `truncate_diff` is public API on `assay_core::gate`; `truncate_head_tail`/`TruncationResult` remain pub(crate)

v0.4.0 decisions (from 37-01):
- Command-not-found is Warning severity (binary may exist in execution env but not validation env)
- Whitespace-only prompt treated same as missing for AgentReport validation
- Cycle detection only runs when spec has dependencies (avoids unnecessary full-directory scan)

v0.4.0 decisions (from 37-02):
- spec_validate uses fully qualified type paths (no import additions needed)
- FeatureSpec parse/validation errors fall through to domain_error (not converted to ValidationResult)

v0.4.0 decisions (from 38-01):
- spec_get resolved block uses inline `serde_json::json!()` (no dedicated response struct)
- spec tier in timeout cascade is always null (per-criterion timeout is visible in criteria array)
- Resolved block built once before match, inserted into each branch

v0.4.0 decisions (from 38-02):
- Growth rate uses last cumulative token count divided by turn count for average (simple, stable)
- `estimate_tokens` does both tail-read (usage) and full parse (growth rate)

v0.4.0 decisions (from 40-01):
- WorkSession uses `String` for id (ULID stored as string for schemars compatibility)
- No `deny_unknown_fields` on WorkSession (mutable document, evolves in later phases)
- ulid dependency wired into assay-core only (ID generation is business logic)

v0.4.0 decisions (from 39-02):
- `budget_context` uses passthrough optimization when content fits (avoids pipeline overhead)
- Cupel pipeline method is `.run()` returning `Vec<ContextItem>` (not `.execute()`/`ScoredItem`)
- `ContextBudget` variant added to `AssayError` for cupel error mapping
- tokens module stays `pub(crate)` -- budgeting accesses via `super::tokens`

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

Last session: 2026-03-15
Stopped at: Completed 40-01-PLAN.md
Resume file: None
