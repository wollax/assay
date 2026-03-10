# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-08)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.3.0 Orchestration Foundation

## Current Position

Phase: 32 — CLI Polish
Plan: 4 of 4
Status: Complete
Last activity: 2026-03-10 — Completed 32-04-PLAN.md (enforcement dedup)

Progress: v0.3.0 [██████████████░░] 88% (7/8 phases)

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 15 (11-25) | 52 | 100% (shipped) |
| v0.3.0 | 8 (26-33) | 43 | 88% |

## Phase Checklist

- [x] Phase 26: Structural Prerequisites (CORE-01, CORE-05)
- [x] Phase 27: Types Hygiene (TYPE-01 through TYPE-06)
- [x] Phase 28: Worktree Manager (ORCH-01 through ORCH-07)
- [x] Phase 29: Gate Output Truncation (GATE-01 through GATE-05)
- [x] Phase 30: Core Tech Debt (CORE-02, CORE-03, CORE-04, CORE-06, CORE-07, CORE-08, CORE-09)
- [x] Phase 31: Error Messages (ERR-01, ERR-02, ERR-03)
- [x] Phase 32: CLI Polish (CLI-01 through CLI-08)
- [ ] Phase 33: MCP Validation (MCP-01 through MCP-05)

## Accumulated Context

### Decisions

v0.1.0 decisions archived to .planning/milestones/v0.1.0-ROADMAP.md
v0.2.0 decisions archived to .planning/milestones/v0.2.0-ROADMAP.md

- v0.3.0 targets headless sequential workflow (not full interactive orchestration)
- Concrete Claude Code module, NOT an agent launcher trait (premature abstraction)
- Merge-back pipeline deferred to v0.4.0 (premature without orchestrator)
- Spec provider trait deferred (one implementation = premature abstraction)
- CLI monolith extraction is prerequisite for all v0.3.0 feature work (DONE — Phase 26)
- CLI commands/ module: flat files, one per subcommand group, shared helpers in mod.rs
- Each command module exposes pub(crate) handle() for dispatch
- main.rs is 182 lines (help text attributes tightly coupled to Command enum)
- TUI assay-core dependency verified (Phase 26)
- AssayError::Json variant added; existing Io call sites kept as-is, new code uses constructors
- Sub-enum error pattern for new error categories (WorktreeError, etc.)
- Zero new workspace dependencies (hard constraint from research)
- Launcher, session record, gate_evaluate, TUI viewer, composable gates, spec preconditions, gate history summary — all deferred to v0.4.0+
- Head/tail truncation ratio 1:2 (33% head, 67% tail) with marker as overhead (GATE-01)
- Levenshtein fuzzy match threshold: distance <= 2 AND distance <= name.len() / 2
- SpecNotFoundDiagnostic is a separate error variant (not enriching existing SpecNotFound)
- ERR-01 detects exit codes 127/126 (not io::ErrorKind) since commands spawn via sh -c

### Pending Issues

19 open issues remain from v0.2.0 triage (see .planning/issues/TRIAGE-SUMMARY.md)
9 new issues from Phase 28 PR review (worktree module, GitHub #78-#86)

### Blockers

None.

### Next Actions

Phase 32 complete (all 4 plans). Ready for Phase 33: MCP Validation.
