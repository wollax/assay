# Project

## What This Is

Assay is a spec-driven quality gate system for AI coding agents, built in Rust. Agents write code in isolated git worktrees; Assay evaluates their output against structured specs using dual-track gates (deterministic commands + agent-evaluated criteria), tracks work sessions, and manages the merge-back lifecycle. It's consumed via an MCP server (18 tools), a CLI, and a TUI.

## Core Value

Structured, repeatable quality evaluation of AI-generated code changes against explicit specs — with full lifecycle management from worktree creation through gate evaluation to merge proposal.

## Current State

v0.4.0 on main. ~17K lines of Rust across 6 crates. Ships:

- **assay-types**: Serializable DTOs — Spec, Criterion, GateRunRecord, GateEvalContext, WorkSession, WorktreeMetadata, Config, HarnessProfile, PromptLayer, SettingsOverride, HookContract, etc.
- **assay-core**: Domain logic — spec loading/validation, gate evaluation (command + agent), run history, worktree CRUD, work session lifecycle, merge checking, guard daemon, context diagnostics/pruning, checkpoint extraction, evidence formatting
- **assay-harness**: Agent harness adapters — prompt builder (`build_prompt`), settings merger (`merge_settings`), Claude Code adapter (`generate_config`, `write_config`, `build_cli_args`)
- **assay-cli**: CLI binary — init, spec, gate, worktree, context, checkpoint, guard, mcp, run subcommands (extracted into `commands/` modules)
- **assay-mcp**: MCP server — 19 tools (spec_list/get/validate, gate_run/evaluate/report/finalize/history, worktree_create/list/status/cleanup, session_create/get/update/list, merge_check, context_diagnose, run_manifest)
- **assay-tui**: TUI binary — skeleton (42-line placeholder)

Key patterns: free functions (zero traits), sync core with async surfaces, atomic file writes, `deny_unknown_fields` on persisted types, schema registry via `inventory`, subprocess execution via `std::process::Command`.

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

- [x] M001: Single-Agent Harness — manifest → worktree → agent launch → gate → merge propose (all 7 slices complete, UAT pending real Claude Code invocation)
- [ ] M002: Multi-Agent Orchestration — DAG executor, parallel sessions, sequential merge
- [ ] M003: Conflict Resolution & Polish — AI conflict resolution, additional adapters, type unification
