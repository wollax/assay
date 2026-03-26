# Project

## What This Is

Assay is a spec-driven development platform for AI-augmented workflows, built in Rust. Developers describe what they want to build; Assay's guided wizard breaks the work into milestones and verifiable chunks, generates gate criteria, drives an AI agent through each chunk, evaluates output using dual-track quality gates (deterministic commands + agent-evaluated criteria), and optionally creates gate-gated PRs when all criteria pass. It ships as a TUI (the primary surface), a CLI, an MCP server, and plugins for Claude Code, Codex, and OpenCode.

## Core Value

A beginning developer installs Assay, describes a feature, and gets a structured, gated development cycle — spec → agent → gates → PR — without knowing how to write acceptance criteria or orchestrate agents.

## Current State

v0.9.0-dev. M001–M010 complete. ~25K lines of Rust across 6 crates. 1481+ tests passing. 67 requirements validated, 0 active. `assay-tui` is a full Ratatui application with dashboard, in-TUI authoring wizard, spec browser, provider configuration, agent spawning with live output streaming, slash command overlay, MCP server configuration panel, PR status badges with background polling, and gate history analytics screen. Full OpenTelemetry tracing stack: structured leveled events (zero eprintln!), pipeline and orchestration spans, JSON file trace export under `.assay/traces/`, feature-flagged OTLP export, and W3C TRACEPARENT subprocess propagation.

**M001 (complete):** Single-agent harness end-to-end — manifest → worktree → agent launch → gate evaluation → merge proposal. 19 requirements validated.

**M002 (complete):** Multi-agent orchestration — DAG executor, parallel sessions, sequential merge runner, multi-adapter harness (Claude Code, Codex, OpenCode), scope enforcement. 24 requirements validated, 1183 tests.

**M003 (complete):** Conflict resolution & polish — AI conflict resolution with full audit trail and post-resolution validation. 27 requirements validated, 1222 tests.

**M004 (complete):** Coordination modes — OrchestratorMode enum, Mesh executor (parallel + roster + file-based peer messaging + SWIM membership), Gossip executor (coordinator + knowledge manifest + PromptLayer injection). 32 requirements validated, 1271 tests.

**M005 (complete):** Spec-driven development core — S01 delivered Milestone/ChunkRef/MilestoneStatus types, atomic I/O, milestone_list/milestone_get MCP tools. S02 delivered the full development cycle state machine: `cycle_status`/`cycle_advance`/`chunk_status` MCP tools, guarded phase transitions (Draft→InProgress→Verify→Complete), `assay milestone status`/`advance` CLI subcommands. S03 delivered the guided authoring wizard: `assay_core::wizard` module (create_from_inputs, create_milestone_from_params, create_spec_from_params), `assay plan` CLI with dialoguer TTY guard, `milestone_create`/`spec_create` MCP tools. S04 delivered gate-gated PR creation: `pr_check_milestone_gates`/`pr_create_if_gates_pass`, `assay pr create` CLI, `pr_create` MCP tool. S05 delivered the Claude Code plugin upgrade: three new skills (`/assay:plan`, `/assay:status`, `/assay:next-chunk`), updated CLAUDE.md, cycle-aware `cycle-stop-check.sh` Stop hook, updated PostToolUse reminder with active chunk name, plugin version 0.5.0. S06 delivered the Codex plugin: AGENTS.md workflow guide + 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan). 43 requirements validated (R039–R048). 1333 tests.

**M006 (complete):** TUI as primary surface — S01 fixed binary name (`[[bin]] name = "assay-tui"`) + live dashboard from `milestone_scan`. S02 delivered in-TUI authoring wizard (`WizardState` pure state machine, `draw_wizard` popup, `wizard_round_trip` integration test). S03 delivered spec browser (`MilestoneDetail` + `ChunkDetail` screens, `join_results` criterion/gate-result join, Esc chains). S04 delivered provider configuration (`ProviderKind`+`ProviderConfig` in `assay-types`, `config_save` atomic write, `Screen::Settings` full-screen view, 5 settings integration tests including restart-persistence). S05 delivered integration polish (`?` help overlay, persistent status bar, global `area: Rect` layout split, `Event::Resize` fix, `just ready` green). 4 new requirements validated (R049–R052). 1367 tests.

**M007 (complete):** TUI agent harness — S01 delivered channel-based event loop (`TuiEvent`), `launch_agent_streaming`, `Screen::AgentRun` with `r` key spawning agents. S02 delivered provider dispatch (`provider_harness_writer` routing Anthropic/Ollama/OpenAI), Settings screen model input fields. S03 delivered slash command overlay (`/` key, tab completion, `/gate-check`/`/status`/`/next-chunk`/`/pr-create` commands). S04 delivered MCP server configuration panel (`m` key, add/delete/save servers to `.assay/mcp.json`). R053, R055 validated. 1400+ tests.

**M008 (complete):** PR Workflow + Plugin Parity — S01 delivered advanced PR creation with labels, reviewers, and body templates from milestone TOML. S02 delivered TUI PR status panel with background `gh` polling (60s interval) showing state/CI/review badges. S03 delivered OpenCode plugin with AGENTS.md + 5 skills matching Codex parity. S04 delivered gate history analytics engine (`compute_analytics`, `assay history analytics` CLI with text tables and `--json`). S05 delivered TUI analytics screen (`a` key from Dashboard, failure frequency heatmap with color-coded rates, milestone velocity table). R057, R058, R059 validated. 1400+ tests.

**M009 (complete):** Observability — S01 replaced all eprintln! with structured tracing macros, established init_tracing()/TracingConfig/TracingGuard layered subscriber foundation. S02 added #[instrument] on 5 pipeline functions and info_span! on 6 stage blocks. S03 instrumented DAG/Mesh/Gossip orchestration with root+session spans and cross-thread parenting in std::thread::scope workers. S04 built custom JsonFileLayer writing Vec<SpanData> JSON files to `.assay/traces/` plus `assay traces list` and `assay traces show <id>` CLI. S05 added feature-flagged OTLP exporter (telemetry feature, http-proto+hyper-client transport) with TracingGuard::drop() shutdown and W3C TRACEPARENT injection in both subprocess launch paths. Default build has zero OTel deps. R027, R060–R065 validated. 1400+ tests.

**M010 (complete):** Pluggable State Backend — S01 delivered StateBackend trait (7 sync methods, object-safe), CapabilitySet flags struct, LocalFsBackend skeleton, StateBackendConfig enum (LocalFs + Custom), contract tests proving trait object construction, D149 documenting the deliberate D001 exception. R071 validated. S02 delivered LocalFsBackend real method bodies (push_session_event with atomic tempfile-rename, read_run_state, save_checkpoint_summary, send_message, poll_inbox, annotate_run), Arc<dyn StateBackend> on OrchestratorConfig (manual Clone/Debug impls), replacement of all 11 persist_state() callsites across executor/mesh/gossip, RunManifest.state_backend field with backward-compat serde, schema snapshot split for orchestrate/non-orchestrate feature variants, CLI/MCP construction sites updated with explicit LocalFsBackend. R072/R073 validated. S03 delivered CapabilitySet graceful degradation: orchestrator checks supports_messaging before mesh routing and supports_gossip_manifest before knowledge manifest injection, NoopBackend test helper, two degradation tests. R074 validated. S04 delivered smelt-agent plugin: AGENTS.md (45 lines) + 3 skills (run-dispatch, backend-status, peer-message) teaching smelt workers the backend-aware API surface. R075 validated. 1481+ tests.

Crates:

- **assay-types**: Serializable DTOs — Spec, Criterion, GateRunRecord, GateEvalContext, WorkSession, WorktreeMetadata, Config, HarnessProfile, PromptLayer, SettingsOverride, HookContract, OrchestratorStatus, SessionRunState, FailurePolicy, MergeStrategy, ScopeConfig, ScopeViolation, etc.
- **assay-core**: Domain logic — spec loading/validation, gate evaluation (command + agent), run history, worktree CRUD, work session lifecycle, merge checking/execution, guard daemon, context diagnostics/pruning, checkpoint extraction, evidence formatting, DAG validation, parallel session orchestration, sequential merge runner
- **assay-harness**: Agent harness adapters — prompt builder, settings merger, Claude Code/Codex/OpenCode adapters, scope enforcement
- **assay-cli**: CLI binary — init, spec, gate, worktree, context, checkpoint, guard, mcp, run, harness, milestone, plan, pr subcommands
- **assay-mcp**: MCP server — 30 tools (spec, gate, worktree, session, merge, context, orchestrate, milestone, cycle, pr)
- **assay-tui**: TUI binary — full Ratatui app (M006+M007): App+Screen state machine (Dashboard/NoProject/Wizard/MilestoneDetail/ChunkDetail/Settings/AgentRun/McpPanel/LoadError), live dashboard, in-TUI authoring wizard, spec browser, provider config, agent spawning with live output streaming, slash command overlay, MCP server config panel, help overlay, status bar

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
- [x] M006: TUI as Primary Surface — full Ratatui TUI: dashboard, wizard, spec browser, provider config, help overlay, status bar (complete, R049–R052 validated, 1371+ tests)
- [x] M007: TUI Agent Harness — TUI spawns and controls AI agents, provider abstraction (Anthropic/OpenAI/Ollama), MCP management, slash commands (complete, R053/R055 validated, 1400+ tests)
- [x] M008: PR Workflow + Plugin Parity — advanced PR automation, OpenCode plugin, history analytics (complete, R057–R059 validated, 1400+ tests)
- [x] M009: Observability — structured tracing foundation, pipeline + orchestration span instrumentation, JSON file trace export, OTLP export + TRACEPARENT context propagation (complete, R027/R060–R065 validated, 1400+ tests)
- [x] M010: Pluggable State Backend — All 4 slices complete. StateBackend trait (7 sync methods, object-safe, D149 exception to D001), CapabilitySet flags, LocalFsBackend full implementation, StateBackendConfig schema locked, RunManifest.state_backend backward-compat field, Arc<dyn StateBackend> on OrchestratorConfig, all persist_state() callsites replaced, graceful degradation for messaging/gossip capabilities, smelt-agent plugin with AGENTS.md + 3 skills. R071–R075 validated. 1481+ tests.
