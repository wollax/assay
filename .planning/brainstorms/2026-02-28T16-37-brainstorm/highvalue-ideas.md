# High-Value Feature Proposals for Assay

**Explorer:** explorer-highvalue
**Date:** 2026-02-28
**Status:** Initial proposals

---

## 1. Spec-Driven Development Engine

**What:** A rich specification authoring and tracking system that replaces the current bare-bones `Spec { name, description }` with a full-featured specification model. Specs would include structured acceptance criteria (as a checklist of verifiable conditions), decomposed tasks, dependency graphs between specs, status lifecycle (Draft → Active → Implementing → Review → Done), and Markdown-based authoring with TOML frontmatter. The CLI would support `assay spec new`, `assay spec list`, `assay spec check` etc. Specs would live as `.assay/specs/*.md` files in the project directory.

**Why:** This is the *foundational* feature. Assay's entire identity is "spec-driven workflows." Without a real spec engine, there's no product — just empty types. Spec files as Markdown+TOML means agents AND humans can author and read them natively. Structured acceptance criteria enable automated verification (gates can check criteria off). This is the primitive that every other feature builds on.

**Scope:** Medium-Large. Requires: new types in assay-types (AcceptanceCriterion, SpecStatus, TaskBreakdown), spec parsing/validation in assay-core, file I/O with serde + TOML frontmatter parser, CLI subcommands, error types. Estimate: ~2-3 focused milestones.

**Risks:**
- Over-engineering the spec format before understanding real usage patterns. Could end up with a schema nobody wants to write by hand.
- Markdown+frontmatter parsing is fiddly — edge cases with TOML in YAML-like contexts.
- Tension between human-readable Markdown specs and machine-parseable structured data.

---

## 2. Programmable Gate Evaluation Framework

**What:** Replace the static `Gate { name, passed: bool }` with a programmable gate system where gates define *evaluation strategies* that are executed at runtime. Gate types would include:
- **Command gates:** Run a shell command (e.g., `cargo test`, `cargo clippy`), pass/fail based on exit code
- **Threshold gates:** Check a numeric metric against a threshold (e.g., test coverage ≥ 80%)
- **File gates:** Assert file existence, content patterns, or schema conformance
- **Composite gates:** AND/OR/ALL-OF combinations of other gates
- **Agent gates:** Invoke an AI agent to evaluate work quality against criteria (the killer differentiator)

Gates produce structured `GateResult` with status (Pass/Fail/Skip/Error), evidence (stdout, metrics, agent reasoning), duration, and timestamp. Gates are defined in config and attached to workflow transitions.

**Why:** Gates are Assay's enforcement mechanism — the thing that turns "suggestions" into "guardrails." Without programmable gates, Assay is just another task tracker. Command gates provide immediate value (run your CI checks locally). Agent gates are the unique differentiator: imagine a gate that asks Claude to review whether the implementation actually satisfies the spec's acceptance criteria before allowing the workflow to proceed. This is where "agentic development kit" becomes real.

**Scope:** Large. Requires: gate evaluation trait/function system in assay-core, command execution (tokio process), result types in assay-types, gate definition in config, CLI commands to run gates manually. Agent gates are a second phase requiring the Agent Protocol (idea #3). Estimate: ~2-3 milestones for core gates, +1 for agent gates.

**Risks:**
- Shell command execution has security implications (injection, arbitrary code).
- Agent gates depend on external AI services — network failures, cost, latency, nondeterminism.
- Gate definitions could become their own DSL/config language that's hard to maintain.
- Balancing "run fast locally" with "run thorough checks" — developers will skip gates if they're slow.

---

## 3. Agent Protocol Layer (MCP Server)

**What:** Expose Assay's capabilities as a Model Context Protocol (MCP) server so any AI coding agent can interact with the spec-gate-review lifecycle programmatically. The MCP server would provide tools like:
- `assay/spec/get` — read current spec and acceptance criteria
- `assay/spec/check-criterion` — mark a criterion as satisfied with evidence
- `assay/gate/run` — trigger gate evaluation
- `assay/gate/status` — check current gate state
- `assay/review/submit` — submit work for review with self-assessment
- `assay/workflow/status` — get current workflow state
- `assay/workflow/advance` — request workflow state transition

The MCP server runs alongside the agent (or is embedded via the plugins) and maintains the source-of-truth for project state. Each agent plugin (claude-code, codex, opencode) would configure this MCP server connection.

**Why:** This is what makes Assay an *agentic* development kit rather than just a CLI for humans. Right now, the plugins are empty shells. An MCP server is the standard way to give AI agents structured access to tools. With this, Claude Code could automatically read the spec before starting work, check gate status as it implements, and submit for review when done — all without human prompting. It turns Assay from "a CLI that developers use" into "a protocol that agents follow."

**Scope:** Large. Requires: MCP server implementation (likely using `rmcp` or similar Rust MCP SDK), tool definitions, state management, transport layer (stdio for embedded, HTTP for standalone). Plugin configurations for each agent. Estimate: ~2-3 milestones.

**Risks:**
- MCP is still evolving — the spec could change and break compatibility.
- Embedding an MCP server adds runtime complexity and dependency weight.
- Agents might not reliably use MCP tools even when available — adoption depends on prompt engineering in the plugins.
- State synchronization between CLI/TUI usage and MCP server interactions.

---

## 4. Workflow State Machine with Audit Trail

**What:** Replace the current flat `Workflow { name, specs, gates }` with a real state machine where workflows define:
- **Phases** with defined transitions (e.g., Specify → Implement → Verify → Review → Ship)
- **Transition guards** — gates that must pass before a transition is allowed
- **Event log** — every state change, gate evaluation, and review decision is recorded with timestamp, actor (human or agent), and evidence
- **Rollback support** — workflows can revert to previous phases when gates fail or reviews reject

The workflow engine would be the orchestration core that ties specs, gates, and reviews together into an enforceable process. Workflow definitions could be TOML-based with a set of built-in templates (simple, standard, strict).

**Why:** The current Workflow type is a data bag with no behavior. A real state machine gives Assay its *process enforcement* capability — the thing that prevents agents from skipping steps. The audit trail is critical for trust: when an AI agent says "I've implemented the spec and all gates pass," you need a verifiable record of what actually happened. Built-in templates lower the barrier to adoption ("just use the standard workflow").

**Scope:** Medium-Large. Requires: state machine implementation in assay-core, event/audit types in assay-types, persistence for workflow state and events, workflow definition format, CLI commands for workflow management. Estimate: ~2 milestones.

**Risks:**
- State machines are deceptively complex — handling edge cases (concurrent transitions, crashed evaluations, partial rollbacks) is hard.
- Too-rigid workflows will frustrate developers who want flexibility.
- The audit trail can grow large for long-running workflows — needs a storage/pruning strategy.
- Defining the "right" default workflow templates requires domain expertise in spec-driven development.

---

## 5. Structured Review System with Rubrics

**What:** Evolve the binary `Review { approved: bool }` into a multi-dimensional review system with:
- **Rubrics** — named sets of evaluation criteria (e.g., "correctness," "completeness," "code quality," "test coverage," "spec conformance")
- **Scored reviews** — each criterion gets a rating (Pass/Partial/Fail) with written evidence/comments
- **Weighted aggregation** — rubric criteria have configurable weights; overall review status computed from weighted scores
- **Multi-reviewer support** — both human and agent reviewers, with different rubric weights per reviewer type
- **Review lifecycle** — Pending → In Review → Changes Requested → Approved, with threaded discussion

**Why:** Binary approval is insufficient for quality enforcement. A rubric-based system lets you define *what* quality means for your project and measure it consistently across reviews. Multi-reviewer support is the agent integration story: an AI agent can do a first-pass review against the rubric, human reviewers focus on the criteria that need human judgment. This also produces structured data that can feed back into improving specs and gates over time (e.g., "criteria X fails 80% of reviews — the spec is unclear").

**Scope:** Medium. Requires: rubric types in assay-types, review evaluation logic in assay-core, rubric definition in config, CLI commands for review management. Multi-reviewer and lifecycle add complexity. Estimate: ~1-2 milestones for core, +1 for multi-reviewer.

**Risks:**
- Rubric fatigue — too many criteria make reviews tedious and developers will rubber-stamp them.
- Weighted scoring can produce unintuitive results ("it passed overall but failed on correctness?").
- Agent reviews are only as good as the agent's ability to evaluate code — false positives/negatives erode trust.
- Feature creep: review systems can expand endlessly (threading, reactions, assignments, etc.).

---

## 6. Plugin SDK with Event-Driven Hooks

**What:** A proper SDK that agent plugins use to integrate with Assay's lifecycle. The SDK defines:
- **Event types** — SpecCreated, SpecUpdated, GateEvaluated, GatePass, GateFail, ReviewSubmitted, ReviewApproved, WorkflowTransition, etc.
- **Hook registration** — plugins register handlers for events they care about
- **Agent-specific adapters** — translate Assay events into each agent's native format:
  - Claude Code: hooks.json entries, skill triggers, CLAUDE.md context injection
  - Codex: AGENTS.md updates, skill triggers
  - OpenCode: opencode.json tool definitions, command registrations
- **Bidirectional:** plugins can both *react to* events and *trigger* Assay actions

The SDK would be a thin Rust crate (`assay-plugin-sdk`) that compiles to each target's native format, plus TypeScript bindings for OpenCode.

**Why:** The three plugin directories are currently empty. A Plugin SDK turns them from aspirational folders into functional integrations. Event-driven hooks mean agents automatically get context about what's happening (a gate failed, a review was requested) without requiring manual invocation. This is the "glue" between Assay's core engine and the AI agents it's designed to work with. Without it, Assay is a standalone CLI that agents happen to exist alongside — not an "agentic development kit."

**Scope:** Large. Requires: event system in assay-core, SDK crate, per-agent adapter code, plugin template generators. Cross-language support (Rust + TypeScript) adds complexity. Estimate: ~2-3 milestones.

**Risks:**
- Each agent system (Claude Code, Codex, OpenCode) has different plugin models — maintaining three adapters is expensive.
- Event-driven systems can be hard to debug (what fired? what was the event order?).
- The SDK adds a public API surface that constrains future changes — premature stabilization risk.
- Agent plugin ecosystems are changing rapidly — today's plugin model may be deprecated tomorrow.

---

## 7. Project Configuration & Persistence Layer

**What:** A file-based persistence layer that gives Assay durable state. This includes:
- **Project init** — `assay init` creates an `.assay/` directory with config, spec templates, and default workflow
- **Configuration format** — `.assay/config.toml` for project settings (workflow templates, gate definitions, rubric defaults, agent preferences)
- **Spec storage** — `.assay/specs/` directory with Markdown+frontmatter spec files
- **State storage** — `.assay/state.json` for current workflow state, gate results, review history
- **Schema generation** — auto-generate JSON schemas from assay-types to `.assay/schemas/` for editor validation
- **Git-friendly** — all formats are text-based, merge-friendly, and `.gitignore`-aware (state may be excluded)

**Why:** Without persistence, Assay is stateless — every invocation starts from scratch. You can't track which gates have passed, what the current workflow state is, or what reviews have been submitted. The `.assay/` directory convention makes it a per-project tool (like `.git/`). TOML config is the Rust ecosystem standard and human-readable. Git-friendliness means specs and configs can be version-controlled and reviewed in PRs, while ephemeral state stays local.

**Scope:** Medium. Requires: config parsing (TOML), file I/O in assay-core, directory structure conventions, init command in CLI, schema generation from schemars. Estimate: ~1-2 milestones.

**Risks:**
- File format decisions are hard to change later — early choices lock in the UX.
- `.assay/` directory conflicts with other tools or naming conventions.
- State files can diverge from reality (stale state after manual edits, git operations, etc.).
- TOML has limitations for complex nested config — may need to support multiple formats.

---

## Summary & Prioritization

| # | Feature | Strategic Value | Prerequisite For | Effort |
|---|---------|----------------|-------------------|--------|
| 7 | Config & Persistence | Foundation | Everything | Medium |
| 1 | Spec Engine | Core product | Gates, Reviews, Workflows | Medium-Large |
| 2 | Gate Framework | Differentiation | Workflow guards, Agent gates | Large |
| 4 | Workflow State Machine | Process enforcement | End-to-end flow | Medium-Large |
| 5 | Review System | Quality assurance | Multi-reviewer | Medium |
| 3 | Agent Protocol (MCP) | Agent integration | Plugin SDK | Large |
| 6 | Plugin SDK | Ecosystem enablement | Full agent integration | Large |

**Recommended build order:** 7 → 1 → 2 → 4 → 5 → 3 → 6

Persistence first (everything needs file I/O), then the core primitives (specs, gates), then the orchestration layer (workflows), then the quality layer (reviews), then the agent integration layer (MCP + plugins). Each layer builds on the previous.
