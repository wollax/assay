# Assay

## What This Is

An agentic development kit that orchestrates AI coding agents through spec-driven workflows with programmable quality gates and automated merge-back strategies. A full-stack alternative to agtx with quality enforcement as the core differentiator.

## The Problem

AI coding agents (Claude Code, Codex, Gemini, OpenCode) can write code, but there's no tool that:

1. **Enforces quality before merge** — agents produce code with no structured quality gates between their output and the main branch
2. **Manages the merge-back workflow** — worktree isolation is easy; getting agent work back into main safely (conflict resolution, gate enforcement, branch strategy) is unsolved
3. **Works with existing spec systems** — teams already use Kata, Spec Kit, OpenSpec, or custom workflows; they shouldn't have to abandon them to get orchestration

Assay solves all three.

## The Vision

A daemon/orchestrator that manages N concurrent spec-work-gate-merge loops across projects:

1. **Spec provider** reads a spec (from any pluggable source)
2. **Orchestrator** creates a worktree, launches an agent in a tmux session
3. **Agent** implements against the spec in isolation
4. **Gates** run dual-track criteria: deterministic (shell commands, tests) + agent-evaluated (natural-language assertions)
5. **Merge agent** handles conflict resolution and creates a feature branch
6. **Orchestrator** enforces final quality gate before merging to main
7. **Human** supervises via TUI dashboard OR participates as an actor alongside agents

The human controls merge approval (configurable: `autonomous: false` by default).

## Core Differentiators

### 1. Dual-Track Quality Gates

No existing tool combines:
- **Deterministic criteria:** Shell commands, test suites, linter checks — binary, reproducible, cheap
- **Agent-evaluated criteria:** Natural-language assertions verified by AI — nuanced, context-aware, handles what tests can't express

Gates are programmable, composable (AND/OR), and produce structured results with evidence.

### 2. Merge-Back Orchestration

Assay owns the branch strategy end-to-end:
- Per-agent worktrees spawned automatically in configurable location
- Feature-merge agent handles conflict resolution
- Quality gates enforced before any merge to main
- Branch strategy is configurable (feature branches, trunk-based with short-lived branches, etc.)

### 3. Pluggable Spec Workflows

Spec providers are adapters — Assay doesn't own the spec format:
- **Built-in default:** Minimal, opinionated spec format for quick start
- **Pluggable:** Kata, Spec Kit, OpenSpec, custom implementations
- **Spec provider trait:** Defines what Assay needs from any spec system (read specs, check criteria, report status)

Reference architecture: [agtx](https://github.com/fynnfluegge/agtx) — similar orchestration patterns (worktrees, tmux, kanban, multi-agent) but without quality gates or merge-back workflow.

## Architecture

### Workspace Layout

```
crates/
  assay-types  →  Shared serializable types (serde, schemars). No business logic.
  assay-core   →  Domain logic: specs, gates, reviews, workflows. Depends on assay-types.
  assay-cli    →  CLI binary (clap). Depends on assay-core.
  assay-tui    →  TUI binary (ratatui + crossterm). Depends on assay-core.
```

### Dependency Graph

```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
```

The `assay-mcp` crate provides MCP server functionality. Future crates may include `assay-daemon` (orchestrator), but these are not scoped yet.

### Key Abstractions

- **Spec:** Declarative specification of work with acceptance criteria — sourced from any provider
- **SpecProvider:** Trait/interface for pluggable spec systems (Kata, Spec Kit, OpenSpec, built-in, custom)
- **Gate:** Programmable quality checkpoint with dual-track criteria (deterministic + agent-evaluated)
- **GateResult:** Structured pass/fail with evidence, duration, timestamp
- **Workflow:** Pipeline orchestrating spec → work → gate → review → merge
- **Session:** A running agent in a worktree/tmux pane, working against a spec
- **Orchestrator:** Daemon managing N concurrent sessions across projects

## Surfaces

- **CLI** (`assay-cli`) — Human-facing commands for project init, spec management, gate execution, workflow status
- **TUI** (`assay-tui`) — Dashboard for supervising multiple sessions/projects simultaneously
- **MCP Server** (`assay-mcp`) — Machine-facing protocol so agents interact with Assay programmatically
- **Plugins** — Installable integrations for specific agentic AI systems (Claude Code, Codex, OpenCode)
- **IDE** (TBD) — Visual interface

## Technical Stack

- **Language:** Rust (2024 edition, stable)
- **Serialization:** serde + schemars (JSON Schema generation)
- **CLI:** clap 4
- **TUI:** ratatui 0.30 + crossterm
- **Error handling:** thiserror
- **Async:** tokio (for daemon/orchestrator)
- **Build:** cargo workspace, just task runner, mise for tooling, cargo-deny for auditing
- **Process management:** tmux for agent sessions, git worktrees for isolation

## Conventions

- Lean towards functional and declarative patterns
- Use workspace dependencies from root `Cargo.toml` — never add deps to individual crates without adding to workspace first
- Types shared between crates belong in `assay-types`
- Business logic belongs in `assay-core`
- Binary crates are thin wrappers that delegate to `assay-core`
- Run `just ready` before considering work complete

## Current State

**Shipped:** v0.1.0 Proof of Concept (2026-03-02)

5,028 lines of Rust across 5 crates (types, core, cli, tui, mcp). 119 tests. Claude Code plugin with MCP integration, skills, and hooks.

**Next milestone:** TBD

## Requirements

### Validated

- ✓ Workspace structure with 4 crates (types, core, cli, tui) — existing
- ✓ Build toolchain (just, mise, cargo-deny, rustfmt, clippy) — existing
- ✓ Serde + schemars derives on domain types — existing
- ✓ Clap CLI skeleton — existing
- ✓ Ratatui TUI skeleton — existing
- ✓ Error type foundation: unified AssayError with thiserror, `#[non_exhaustive]` — v0.1.0
- ✓ Domain model hardening: GateKind enum, GateResult with evidence, types as pub DTOs — v0.1.0
- ✓ Schema generation pipeline: standalone binary + `just schemas` — v0.1.0
- ✓ Config loading: free functions in assay-core, TOML only — v0.1.0
- ✓ Spec + config validation: free functions in assay-core, trim-then-validate — v0.1.0
- ✓ Gate evaluation: command gates, sync, explicit working_dir, structured GateResult — v0.1.0
- ✓ CLI subcommands: init, gate run, spec show/list, mcp serve — v0.1.0
- ✓ MCP server: stdio via rmcp, 3 tools (spec_get, spec_list, gate_run) — v0.1.0
- ✓ Claude Code plugin: .mcp.json, skills, hooks, CLAUDE.md — v0.1.0

### Active

(None — next milestone not yet defined)

### Future

- [ ] Domain model redesign: spec provider trait, workflow phases
- [ ] Pluggable spec provider interface with built-in default implementation
- [ ] Programmable gate framework: file, threshold, composite, agent-evaluated
- [ ] Dual-track criteria: agent-evaluated track (production)
- [ ] Per-agent worktree management (create, isolate, clean up)
- [ ] tmux session/pane management for agent lifecycle
- [ ] Orchestrator/daemon managing concurrent sessions
- [ ] Merge-back workflow: feature branch creation, conflict resolution, gate enforcement
- [ ] Branch strategy configuration
- [ ] TUI dashboard for multi-session supervision

### Out of Scope

- Custom spec DSL or embedded language — agents + shell commands replace the need
- Agent marketplace or capability routing — YAGNI at this stage
- Production runtime assertions — Assay is a development tool, not an observability platform
- Formal protocol standardization (ADP) — extract from working software later if adoption warrants
- Multi-reviewer weighted rubrics — simple structured review first

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Full alternative to agtx, not a layer on top | Need to own the full stack to integrate gates and merge-back deeply | ✓ Good |
| Pluggable spec providers, not a single format | Teams already use Kata, Spec Kit, etc.; Assay shouldn't force migration | ✓ Good |
| Dual-track gate criteria (deterministic + agent-evaluated) | Novel combination; category-defining differentiator | ✓ Good — deterministic track shipped v0.1.0, agent track deferred |
| Human approves merges by default (`autonomous: false`) | Trust must be earned; configurable for full automation later | Decided |
| Merge-back via feature-merge agent + orchestrator gates | Agent work in worktrees needs quality enforcement before reaching main | Decided |
| TOML for config, JSON for data exchange | Rust ecosystem convention; schemars generates JSON schemas | ✓ Good |
| Enum dispatch for gates, not trait objects | Simpler, serializable, sufficient until plugin system exists | ✓ Good |
| Start with domain model before any UI/orchestration | Everything consumes types; wrong types means rewriting everything | ✓ Good — types stable through 10 phases |
| Gate is pure config, not mixed config+state | `passed: bool` removed; runtime state belongs to GateResult | ✓ Good |
| assay-mcp as library crate, single `assay` binary | All surfaces through one binary, MCP server started via `assay mcp serve` | ✓ Good — v0.1.0 |
| spawn() + reader threads + try_wait for gate timeout | Command::output() can't enforce timeouts; polling with kill on timeout | ✓ Good — v0.1.0 |
| Skills use MCP tool orchestration, not shell commands | Agent calls MCP tools directly rather than shelling out to CLI | ✓ Good — v0.1.0 |

## Reference Material

- [agtx](https://github.com/fynnfluegge/agtx) — Reference architecture for agent orchestration with worktrees/tmux
- Brainstorm session 1: `.planning/brainstorms/2026-02-28T16-37-brainstorm/SUMMARY.md`
- Brainstorm session 2: `.planning/brainstorms/2026-02-28T17-45-brainstorm/SUMMARY.md`

---
*Last updated: 2026-03-02 after v0.1.0 milestone*
