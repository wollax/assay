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

## Current Milestone: v0.6.2 P0 Cleanup

**Goal:** Resolve 27 P0 issues from post-M024 review findings — process safety, type correctness, serde consistency, and test coverage gaps.

**Target fixes:**

- Process safety: `killpg` for process groups, TOCTOU race fix, stderr capture, relay panic logging, terminal injection prevention
- Type correctness: `Option<When>` ambiguity, `SessionPhase` collision rename, `AfterToolCalls{0}` validation, checkpoint timeout override
- Review findings: S04 (auto-promote test name, session lookup perf) and S05 (README fixes, prompt rename, file-path references, serde consistency)
- Test coverage: gate_sessions eviction, pipeline integration tests, claude_stream edge cases, checkpoint Windows portability

## Current State

**Shipped:** v0.6.1 Conflict Resolution & Polish (2026-04-08)

2266 tests across 57 binaries. Full single-agent pipeline (manifest → worktree → harness → agent → gate → merge), multi-agent orchestration (DAG/mesh/gossip), streaming event pipeline, checkpoint gates, auto-promote, Smelt monorepo integration, Forgejo CI.

**Previous:** v0.4.1 Merge Tools (2026-04-08), v0.4.0 Headless Orchestration (2026-03-15), v0.3.0 (2026-03-10), v0.2.0 (2026-03-08), v0.1.0 (2026-03-02)

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
- ✓ Run history: JSON persistence with atomic writes, retention pruning, CLI viewer — v0.2.0
- ✓ Required/advisory enforcement levels on criteria — v0.2.0
- ✓ Agent gate recording: gate_report MCP tool, GateKind::AgentReport, evaluator metadata — v0.2.0
- ✓ FileExists gate kind wired into evaluate() dispatch — v0.2.0
- ✓ Type system hygiene: serde skip_serializing_if, backward compat, schema snapshots — v0.2.0
- ✓ MCP hardening: timeout, working_dir validation, error envelopes, gate_history tool — v0.2.0
- ✓ CLI hardening: error propagation, exit codes, enforcement-aware streaming — v0.2.0
- ✓ Testing & tooling: MCP handler tests, cargo-deny tightened, dogfooding spec — v0.2.0
- ✓ Token-aware session diagnostics: JSONL parser, bloat categorization, context % — v0.2.0
- ✓ Agent team context protection: checkpointing, pruning engine, guard daemon, overflow recovery — v0.2.0
- ✓ Git worktree lifecycle management (create, list, status, cleanup) — v0.3.0
- ✓ CLI correctness: NO_COLOR/TTY handling, help dedup, enforcement dedup, constants extraction — v0.3.0
- ✓ MCP parameter validation with actionable error messages — v0.3.0
- ✓ Types hygiene: Eq derives, Display impls, doc comments, deny(missing_docs), Criterion dedup — v0.3.0
- ✓ Gate/spec error messages: fuzzy matching, exit code classification, TOML error formatting — v0.3.0
- ✓ Gate output truncation with head+tail capture, UTF-8 safety, and MCP visibility metadata — v0.3.0
- ✓ `gate_evaluate` MCP tool — diff + headless evaluator + finalize in single call — v0.4.0
- ✓ `WorkSession` persistence with phase transitions and MCP tools — v0.4.0
- ✓ `spec_validate` MCP tool with structured diagnostics — v0.4.0
- ✓ Context engine integration for token-budgeted context slicing — v0.4.0
- ✓ MCP observability: warnings field, filtered history, resolved config, growth rate — v0.4.0
- ✓ Correctness: worktree base branch status, session error messages, diff context — v0.4.0
- ✓ Tech debt batch cleanup (120+ issues) — v0.4.0

### Active

- [ ] `merge_check` MCP tool — read-only conflict detection via `git merge-tree` — v0.4.1
- [ ] `merge_propose` MCP tool — PR creation with gate evidence, dry_run support — v0.4.1
- [ ] Worktree fixes: canonical paths, default branch errors, prune failure surfacing — v0.4.1
- [ ] `assay-harness` crate with HarnessProfile and Claude Code adapter — v0.5.0
- [ ] Callback-based agent invocation (closures for control inversion) — v0.5.0
- [ ] Worktree enhancements: orphan detection, collision prevention, session linkage — v0.5.0
- [ ] `RunManifest` with `[[sessions]]` array format — v0.5.0
- [ ] Session vocabulary cleanup (`AgentSession` → `GateEvalContext`) — v0.5.0
- [ ] AgentSession write-through persistence — v0.5.0
- [ ] End-to-end single-agent pipeline — v0.5.0

### Future

- [ ] Multi-agent orchestration: `OrchestratorSession`, DAG executor, MergeRunner — v0.6.0
- [ ] `orchestrate_*` MCP tools (additive, no changes to existing tools) — v0.6.0
- [ ] Harness orchestration layer: scope enforcement, multi-agent prompts — v0.6.0
- [ ] AI conflict resolution via evaluator subprocess — v0.6.1
- [ ] Cupel integration for orchestrated sessions — v0.6.1
- [ ] Codex/OpenCode adapter stubs — v0.6.1
- [ ] `SessionCore` struct composition for type unification — v0.6.1
- [ ] Minimal TUI gate results viewer
- [ ] Composable gate definitions (`gate.extends`)
- [ ] Criteria libraries with `include` field
- [ ] Spec preconditions section
- [ ] Gate history summary with pass/fail rates
- [ ] Full TUI dashboard for multi-session supervision
- [ ] Pluggable spec provider interface with built-in default implementation
- [ ] Trust scores: quantified agent reliability from gate history

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
| Agent gates receive evaluations, not call LLMs | Agents already have LLM access; Assay records results via `gate_report` MCP tool | ✓ Good — v0.2.0 |
| Self-evaluation + audit trail for v0.2, independent evaluator for v0.3 | Trust problem is real but unsolvable without orchestrator; history enables human audit | ✓ Good — v0.2.0 |
| Keep core types domain-agnostic | Gate evaluation and evidence capture should work for any domain, not just code | ✓ Good — v0.2.0 |
| No built-in LLM client | `gate_report` → `gate_evaluate` progression may make it unnecessary; avoid HTTP/API key complexity | ✓ Good — v0.2.0 |
| No SpecProvider trait yet | One implementation = premature abstraction; wait for concrete second provider | ✓ Good — v0.2.0 |
| Cozempic-inspired features in Rust, not Python | Full native performance; avoids Python dependency; aligns with workspace | ✓ Good — v0.2.0 |
| Session diagnostics + team protection appended to v0.2.0 | Orthogonal to gates (phases 20-23); fits "hardening" theme; no disruption to 11-19 | ✓ Good — v0.2.0 |
| Guard daemon with kqueue/inotify, not polling-only | Sub-second reactive recovery for inbox-flood overflow (Cozempic's key insight) | ✓ Good — v0.2.0 |
| Composable pruning strategies, dry-run default | Safety first — never modify without `--execute`; team messages always protected | ✓ Good — v0.2.0 |
| Absorb Smelt orchestration into Assay, Smelt pivots to infrastructure | Assay becomes the complete agent dev platform; Smelt becomes CI/CD for agent work | Decided — v0.5.0 |
| Closures for control inversion, not traits | Zero-trait codebase convention (0 traits across 33k lines); closures are Rust-idiomatic | Decided — v0.5.0 |
| Orchestration as assay-core module, not separate crate | ~2 new modules + extensions, not enough for crate-level separation | Decided — v0.5.0 |
| assay-harness as leaf crate | Adapter implementations depend on core, not vice versa; keeps dep graph clean | Decided — v0.5.0 |
| OrchestratorSession composes WorkSessions | Linear state machine preserved; graph structure lives above it | Decided — v0.6.0 |
| Additive orchestrate_* MCP tools, don't modify existing | Avoids interfield validation ambiguity from optional params on existing tools | Decided — v0.6.0 |

## Reference Material

- [agtx](https://github.com/fynnfluegge/agtx) — Reference architecture for agent orchestration with worktrees/tmux
- Brainstorm session 1: `.planning/brainstorms/2026-02-28T16-37-brainstorm/SUMMARY.md`
- Brainstorm session 2: `.planning/brainstorms/2026-02-28T17-45-brainstorm/SUMMARY.md`
- Brainstorm session 3: `.planning/brainstorms/2026-03-02T20-53-brainstorm/SUMMARY.md`
- Brainstorm session 4: `.planning/brainstorms/2026-03-08T21-22-brainstorm/SUMMARY.md`
- Brainstorm session 5: `.planning/brainstorms/2026-03-15T16-14-brainstorm/SUMMARY.md`
- [Cozempic](https://github.com/Ruya-AI/cozempic) — Reference for token-aware diagnostics and agent team context loss protection

---
*Last updated: 2026-03-15 after v0.5.0 milestone defined*
