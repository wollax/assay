# Project

## What This Is

Assay is a spec-driven development platform for AI-augmented workflows, built in Rust. Developers describe what they want to build; Assay's guided wizard breaks the work into milestones and verifiable chunks, generates gate criteria, drives an AI agent through each chunk, evaluates output using dual-track quality gates (deterministic commands + agent-evaluated criteria), and optionally creates gate-gated PRs when all criteria pass. It ships as a TUI (the primary surface), a CLI, an MCP server, and plugins for Claude Code, Codex, and OpenCode.

## Core Value

A beginning developer installs Assay, describes a feature, and gets a structured, gated development cycle — spec → agent → gates → PR — without knowing how to write acceptance criteria or orchestrate agents.

## Current State

v0.5.0 on main. ~20K lines of Rust across 6 crates. M001–M005 complete. 1333 tests passing.

**M001 (complete):** Single-agent harness end-to-end — manifest → worktree → agent launch → gate evaluation → merge proposal. 19 requirements validated.

**M002 (complete):** Multi-agent orchestration — DAG executor, parallel sessions, sequential merge runner, multi-adapter harness (Claude Code, Codex, OpenCode), scope enforcement. 24 requirements validated, 1183 tests.

**M003 (complete):** Conflict resolution & polish — AI conflict resolution with full audit trail and post-resolution validation. 27 requirements validated, 1222 tests.

**M004 (complete):** Coordination modes — OrchestratorMode enum, Mesh executor (parallel + roster + file-based peer messaging + SWIM membership), Gossip executor (coordinator + knowledge manifest + PromptLayer injection). 32 requirements validated, 1271 tests.

**M005 (complete):** Spec-driven development core — S01 delivered Milestone/ChunkRef/MilestoneStatus types, atomic I/O, milestone_list/milestone_get MCP tools. S02 delivered the full development cycle state machine: `cycle_status`/`cycle_advance`/`chunk_status` MCP tools, guarded phase transitions (Draft→InProgress→Verify→Complete), `assay milestone status`/`advance` CLI subcommands. S03 delivered the guided authoring wizard: `assay_core::wizard` module (create_from_inputs, create_milestone_from_params, create_spec_from_params), `assay plan` CLI with dialoguer TTY guard, `milestone_create`/`spec_create` MCP tools. S04 delivered gate-gated PR creation: `pr_check_milestone_gates`/`pr_create_if_gates_pass`, `assay pr create` CLI, `pr_create` MCP tool. S05 delivered the Claude Code plugin upgrade: three new skills (`/assay:plan`, `/assay:status`, `/assay:next-chunk`), updated CLAUDE.md, cycle-aware `cycle-stop-check.sh` Stop hook, updated PostToolUse reminder with active chunk name, plugin version 0.5.0. S06 delivered the Codex plugin: AGENTS.md workflow guide + 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan). 43 requirements validated (R039–R048). 1333 tests.

Crates:

- **assay-types**: Serializable DTOs — Spec, Criterion, GateRunRecord, GateEvalContext, WorkSession, WorktreeMetadata, Config, HarnessProfile, PromptLayer, SettingsOverride, HookContract, OrchestratorStatus, SessionRunState, FailurePolicy, MergeStrategy, ScopeConfig, ScopeViolation, etc.
- **assay-core**: Domain logic — spec loading/validation, gate evaluation (command + agent), run history, worktree CRUD, work session lifecycle, merge checking/execution, guard daemon, context diagnostics/pruning, checkpoint extraction, evidence formatting, DAG validation, parallel session orchestration, sequential merge runner
- **assay-harness**: Agent harness adapters — prompt builder, settings merger, Claude Code/Codex/OpenCode adapters, scope enforcement
- **assay-cli**: CLI binary — init, spec, gate, worktree, context, checkpoint, guard, mcp, run, harness, milestone, plan, pr subcommands
- **assay-mcp**: MCP server — 30 tools (spec, gate, worktree, session, merge, context, orchestrate, milestone, cycle, pr)
- **assay-tui**: TUI binary — real Ratatui app (M006/S01): App/Screen/WizardState types, live dashboard from milestone_scan, wrapping keyboard navigation, no-project guard; S02–S05 add wizard/spec-browser/settings/polish

Key patterns: free functions (zero traits), sync core with async surfaces, atomic file writes, `deny_unknown_fields` on persisted types, schema registry via `inventory`, shell-out to git CLI, closure-based control inversion (D001).

## Architecture / Key Patterns

```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-types
assay-harness → assay-core ──→ assay-types
```

- Zero-trait convention (closures/callbacks for control inversion)
- Shell out to `git` CLI for all git operations (no git2/gix)
- JSON/TOML file-per-record persistence under `.assay/`
- MCP tools are additive (never modify existing tool signatures)
- TOML for all structured spec and planning artifacts

## Capability Contract

See `.kata/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M001: Single-Agent Harness — manifest → worktree → agent launch → gate → merge propose (complete, 19 requirements validated)
- [x] M002: Multi-Agent Orchestration — DAG executor, parallel sessions, sequential merge, multi-adapter harness, scope enforcement (complete, 24 requirements validated, 1183 tests)
- [x] M003: Conflict Resolution & Polish — AI conflict resolution, audit trail, post-resolution validation (complete, 27 requirements validated, 1222 tests)
- [x] M004: Coordination Modes — Mesh and Gossip modes, mode dispatch, knowledge manifest, SWIM membership (complete, 32 requirements validated, 1271 tests)
- [x] M005: Spec-Driven Development Core — all 6 slices complete (types/I/O/cycle state machine/wizard/PR workflow/plugins). 43 requirements validated (R039–R048), 1333 tests.
- [ ] M006: TUI as Primary Surface — real Ratatui TUI with project dashboard, interactive wizard, spec browser, provider config
- [ ] M007: TUI Agent Harness — TUI spawns and controls AI agents, provider abstraction (Anthropic/OpenAI/Ollama), MCP management, slash commands
- [ ] M008: PR Workflow + Plugin Parity — advanced PR automation, OpenCode plugin, history analytics
