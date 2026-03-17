# Project

## What This Is

Assay is a spec-driven quality gate system for AI coding agents, built in Rust. Agents write code in isolated git worktrees; Assay evaluates their output against structured specs using dual-track gates (deterministic commands + agent-evaluated criteria), tracks work sessions, and manages the merge-back lifecycle. It's consumed via an MCP server (18 tools), a CLI, and a TUI.

## Core Value

Structured, repeatable quality evaluation of AI-generated code changes against explicit specs — with full lifecycle management from worktree creation through gate evaluation to merge proposal.

## Current State

v0.4.0 on main. ~20K lines of Rust across 6 crates. M001 (Single-Agent Harness End-to-End) complete — 7 slices, 19 requirements validated. M002 (Multi-Agent Orchestration & Harness Platform) complete — 6 slices, 5 new requirements validated (24 total), 1183 tests. M003/S01 (AI Conflict Resolution) complete — two-phase merge lifecycle, sync resolve_conflict() subprocess, merge runner wiring, CLI --conflict-resolution flag, MCP parameter; 1216+ tests passing, 25 requirements validated. Ships:

- **assay-types**: Serializable DTOs — Spec, Criterion, GateRunRecord, GateEvalContext, WorkSession, WorktreeMetadata, Config, HarnessProfile, PromptLayer, SettingsOverride, HookContract, OrchestratorStatus, SessionRunState, FailurePolicy, MergeStrategy, ScopeConfig, ScopeViolation, etc.
- **assay-core**: Domain logic — spec loading/validation, gate evaluation (command + agent), run history, worktree CRUD, work session lifecycle, merge checking/execution, guard daemon, context diagnostics/pruning, checkpoint extraction, evidence formatting, DAG validation, parallel session orchestration, sequential merge runner with ordering strategies
- **assay-harness**: Agent harness adapters — prompt builder (`build_prompt`), settings merger (`merge_settings`), Claude Code/Codex/OpenCode adapters (`generate_config`, `write_config`, `build_cli_args`), scope enforcement (`check_scope`, `generate_scope_prompt`)
- **assay-cli**: CLI binary — init, spec, gate, worktree, context, checkpoint, guard, mcp, run, harness (generate/install/update/diff) subcommands
- **assay-mcp**: MCP server — 22 tools (spec_list/get/validate, gate_run/evaluate/report/finalize/history, worktree_create/list/status/cleanup, session_create/get/update/list, merge_check, context_diagnose, run_manifest, orchestrate_run [with conflict_resolution param], orchestrate_status)
- **assay-tui**: TUI binary — skeleton (42-line placeholder)

Key patterns: free functions (zero traits), sync core with async surfaces, atomic file writes, `deny_unknown_fields` on persisted types, schema registry via `inventory`, subprocess execution via `std::process::Command`, closure-based control inversion (D001), two-phase merge lifecycle for conflict resolution (D044), sync subprocess with try_wait timeout polling (D043).

## Architecture / Key Patterns

```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-types
assay-harness → assay-core ──→ assay-types
```

- Zero-trait convention (closures/callbacks for control inversion)
- Shell out to `git` CLI for all git operations (no git2/gix)
- JSON file-per-record persistence under `.assay/`
- MCP tools are additive (never modify existing tool signatures)

## Capability Contract

See `.kata/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M001: Single-Agent Harness — manifest → worktree → agent launch → gate → merge propose (complete, 19 requirements validated)
- [x] M002: Multi-Agent Orchestration — DAG executor, parallel sessions, sequential merge, multi-adapter harness, scope enforcement, MCP tools, end-to-end integration (complete, 24 requirements validated, 1183 tests)
- [ ] M003: Conflict Resolution & Polish — AI conflict resolution (R026 ✓ validated S01), audit trail + validation (R029, R028 — S02 next)
