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

### Milestone Scope Issues (v0.5.0)

Issues pulled into v0.5.0 scope (worktree tech debt to clean up during enhancements):
- "CLI worktree handlers discard error source chain" (from: .planning/issues/open/2026-03-09-worktree-cli-error-chain-lost.md)
- "WorktreeConfig.base_dir uses String where Option<String> is idiomatic" (from: .planning/issues/open/2026-03-09-worktree-config-base-dir-type.md)
- "detect_main_worktree conflates errors with is main worktree" (from: .planning/issues/open/2026-03-09-worktree-detect-main-error-handling.md)
- "WorktreeDirty error contains CLI-specific advice" (from: .planning/issues/open/2026-03-09-worktree-dirty-error-cli-advice.md)
- "ASSAY_WORKTREE_DIR env var not documented in CLI help" (from: .planning/issues/open/2026-03-09-worktree-env-var-undocumented.md)
- "MCP worktree_cleanup tool has no --all equivalent" (from: .planning/issues/open/2026-03-09-worktree-mcp-cleanup-no-all.md)
- "WorktreeInfo and WorktreeStatus missing deny_unknown_fields" (from: .planning/issues/open/2026-03-09-worktree-missing-deny-unknown-fields.md)
- "Git worktree prune failure silently discarded" (from: .planning/issues/open/2026-03-09-worktree-prune-failure-silent.md)
- "Missing test for resolve_worktree_dir with empty base_dir config" (from: .planning/issues/open/2026-03-09-worktree-test-empty-base-dir.md)
- "Missing test for cleanup with force=true on clean worktree" (from: .planning/issues/open/2026-03-09-worktree-test-force-clean.md)
- "Missing test for parse_worktree_list with malformed input" (from: .planning/issues/open/2026-03-09-worktree-test-parse-malformed.md)
- "Worktree path uses to_string_lossy which corrupts non-UTF-8 paths" (from: .planning/issues/open/2026-03-09-worktree-to-string-lossy-non-utf8.md)
- "WorktreeInfo and WorktreeStatus field duplication" (from: .planning/issues/open/2026-03-09-worktree-types-field-duplication.md)
- "WorktreeInfo and WorktreeStatus not registered in schema registry" (from: .planning/issues/open/2026-03-09-worktree-types-not-in-schema-registry.md)
- "Worktree ahead/behind use platform-dependent usize" (from: .planning/issues/open/2026-03-09-worktree-usize-serialization.md)

### Pending Issues

109 remaining open issues in .planning/issues/open/ (non-blocking tech debt carried from v0.2.0–v0.4.0)
See .planning/issues/ for full backlog.

### Blockers

None. v0.4.1 must ship before v0.5.0 work begins.

### Next Actions

Define v0.5.0 requirements, then create roadmap.

### Session Continuity

Last session: 2026-03-15
Stopped at: v0.5.0 milestone definition in progress
Resume file: None
