# Radical Milestone Approaches: Challenging Conventional Sequencing

> Explorer: explorer-radical | Date: 2026-02-28
> Second brainstorm session — focus on FIRST MILESTONE structure, not features

---

## The Conventional Wisdom Being Challenged

The previous brainstorm recommended:
1. Foundation (error types, config, schema gen) → ~10hrs
2. Spec engine → MCP server → Gate framework → Workflow → Review

This is a classic bottom-up, inside-out build. Types first, domain logic second, UI/integrations last. It optimizes for engineering correctness but **may not optimize for proving value, getting feedback, or establishing the core differentiator**.

The question: **Is there a radically different sequencing that gets us to "wow, this is useful" faster?**

---

## Proposal 1: Plugin-First / Outside-In

**What:** Ship the Claude Code plugin as the FIRST deliverable. Not the Rust crates. The plugin contains hardcoded skills, hooks, and commands that implement spec-driven workflow entirely within the agent's own context — no Assay binary needed. The Rust crates are extracted LATER from patterns proven in the plugin.

The plugin would contain:
- A `/spec` skill that creates spec files in a conventional format (markdown + TOML frontmatter)
- A `/gate` skill that runs deterministic checks (shell commands) and evaluates agent criteria
- A PreToolUse hook that blocks commits unless spec gates pass
- Hardcoded workflow: spec → implement → gate → review

**Why this might be better:**
- **Zero compilation barrier.** Users install a plugin, not a Rust binary. Adoption friction is near zero.
- **Agents ARE the users.** Assay's primary consumers are AI agents, not humans typing CLI commands. Building for agents first means building the actual product, not scaffolding.
- **Extracts real patterns.** Instead of speculating about what abstractions are needed, you discover them from working plugin code. The Rust crates become a rigorous extraction, not a speculative design.
- **Ships in days, not weeks.** A Claude Code plugin with 3 skills and 1 hook is a weekend project. A Rust domain model with error types, config loading, and spec validation is weeks.
- **Proves the differentiator immediately.** Dual-track gates (shell commands + agent evaluation) work in a plugin right now. No MCP server needed.

**Scope:** 2-3 days for a functional plugin. No Rust code needed initially.

**Risks:**
- Plugin-specific patterns may not generalize to other agents (Codex, OpenCode)
- No shared state / persistence without the Rust backend
- Plugin skills can't do complex orchestration (no daemon, no worktree management)
- Could create throwaway code that never gets properly extracted
- Users may expect a "real" tool, not "just a plugin"

**Kills:** "Start with domain model" assumption. "Binary crates are the product" assumption. "Types first" assumption.

---

## Proposal 2: Dual-Track Gate Demo (Single Feature, Complete Vertical)

**What:** The first milestone delivers ONE thing and ONE thing only: a working dual-track gate system with a CLI command (`assay check`). No specs, no workflows, no reviews, no config files, no project init. Just:

1. A gate definition format (inline TOML or a single file)
2. `assay check gate.toml` — runs deterministic criteria and reports results
3. `assay check --agent gate.toml` — runs both deterministic AND agent-evaluated criteria
4. Structured output (JSON) showing pass/fail with evidence

Everything else is hardcoded or omitted. The spec is just a reference string in the gate file. The workflow is "run this command." Config is CLI flags.

**Why this might be better:**
- **Proves the differentiator on day 1.** Dual-track gates are Assay's category-defining feature. Ship that, not plumbing.
- **Smallest possible demonstration of value.** A user can run `assay check` and see both deterministic and agent-evaluated results. That's the "aha moment."
- **Forces hard design decisions early.** How do you represent gate results? How do you invoke an agent? How do you handle non-determinism? These are the questions that matter, not "should config be TOML or YAML."
- **Everything else is incremental.** Once `assay check` works, adding specs is "what to check against," adding workflows is "when to check," adding reviews is "what to do with results."
- **Crisp demo for README / landing page.** "Here's what Assay does" in one command.

**Scope:** 1-2 weeks. Error types, gate types, gate evaluation, CLI subcommand, agent invocation.

**Risks:**
- Agent invocation is the hardest part and requires deciding on an agent protocol early
- Without specs, gates float free — users may not understand the larger vision
- Could ship a demo that doesn't compose well with future features
- Agent-evaluated track requires API keys / agent availability — friction for first-time users

**Kills:** "Build foundation first" assumption. "Specs before gates" assumption. "Config system is prerequisite" assumption.

---

## Proposal 3: MCP Server First (Agent-Native Entry Point)

**What:** Skip the CLI. Ship an MCP server (`assay-mcp`) as the first binary. The MCP server exposes 3-4 tools that agents call directly:

- `assay/read_spec` — read a spec file
- `assay/check_gate` — evaluate a gate (deterministic only, initially)
- `assay/submit_review` — submit structured review feedback
- `assay/status` — get current workflow status

The "user interface" is the agent's conversation. Humans interact with Assay through the agent, not through a CLI. The CLI is added later as a human convenience layer on top of the same core logic.

**Why this might be better:**
- **Agents are the primary users.** The MCP server is the REAL interface. The CLI is a secondary convenience. Building the real interface first means you validate the real product.
- **MCP is the distribution channel.** An MCP server gets auto-discovered by Claude Code, Cursor, Windsurf, etc. No installation friction. No PATH issues. No binary distribution.
- **Forces machine-readable design.** When your first consumer is an agent, you design clean JSON interfaces, structured errors, and composable tools. This discipline improves everything built on top.
- **Natural integration testing.** You can test the MCP server by having Claude Code use it. Your test environment is your production environment.
- **Previous brainstorm already moved MCP from position 6 to position 3.** This proposal says: why stop at 3? Make it position 1.

**Scope:** 2-3 weeks. MCP server binary, 3-4 tool handlers, spec file reading, basic gate evaluation.

**Risks:**
- MCP servers are harder to debug than CLIs (no direct human interaction)
- Testing requires an MCP-capable client
- The MCP protocol itself is still evolving — breaking changes possible
- Humans can't use Assay without an agent (chicken-and-egg)
- May over-optimize for agent ergonomics at the expense of human understanding

**Kills:** "CLI is the primary interface" assumption. "Humans are the primary users" assumption. "Build bottom-up" assumption.

---

## Proposal 4: Top-Down Orchestrator Shell

**What:** Build the orchestrator / workflow engine FIRST as a shell that calls stubbed-out components. The orchestrator defines the spec-work-gate-review-merge lifecycle and executes it, but every individual step is a trivial stub or hardcoded implementation. Then fill in the stubs one by one in subsequent milestones.

Think of it like building a car: instead of building the engine first, then the transmission, then the chassis — build the chassis first with a placeholder motor, then swap in real components.

The orchestrator shell would:
1. Read a spec file (hardcoded format, no validation)
2. Create a worktree
3. Launch a tmux session with an agent
4. Wait for agent to signal "done"
5. Run gates (hardcoded: `cargo test`, `cargo clippy`)
6. Report results
7. Clean up worktree

**Why this might be better:**
- **End-to-end flow on day 1.** Users see the entire lifecycle working, even if individual steps are crude.
- **Integration bugs surface immediately.** The hardest problems in Assay are at component boundaries (orchestrator ↔ agent, agent ↔ gate, gate ↔ merge). Building top-down finds these problems first.
- **Natural prioritization.** Once the shell works, you know exactly which stub is the bottleneck. Replace the weakest stub next. This is empirical prioritization, not speculative sequencing.
- **Compelling demo.** "Watch Assay orchestrate an agent implementing a spec with quality gates" is a much better demo than "look at our error types and config loader."
- **agtx already proves this works.** agtx ships a working orchestrator with worktrees + tmux + kanban. Assay can do the same but with gates.

**Scope:** 2-3 weeks. Orchestrator loop, worktree management, tmux integration, stub components, basic gate evaluation.

**Risks:**
- Stub-heavy code creates technical debt that must be paid down
- Orchestrator design without real components may make wrong assumptions
- tmux/worktree management is OS-specific and error-prone
- The "shell" approach can produce a demo that looks impressive but is fragile
- Hard to test: requires git repos, tmux sessions, real agents

**Kills:** "Foundation first" assumption. "Domain model before integration" assumption. "Components before orchestration" assumption.

---

## Proposal 5: Spec-Gate Spec (Assay Eats Its Own Dogfood)

**What:** The first milestone is defined AS an Assay spec. Assay's own development is the test case. Write the spec for Milestone 1 in Assay's own spec format. Define the gates. Then implement just enough of Assay to evaluate those gates against itself. The tool validates its own development process.

Concretely:
1. Write `specs/milestone-1.toml` defining what M1 must deliver
2. Define deterministic gates: `cargo test`, `cargo clippy -D warnings`, `just ready`
3. Define agent-evaluated gates: "CLI help text is clear and discoverable", "Error messages include actionable guidance"
4. Implement the minimum Assay needed to evaluate these gates
5. M1 is "done" when `assay check specs/milestone-1.toml` passes all gates

**Why this might be better:**
- **Dogfooding from day zero.** The tool's first user is its own development process. This creates extreme alignment between what you build and what users need.
- **The spec IS the milestone definition.** No separate project management artifacts. The spec format is proven by the fact that it defines a real milestone.
- **Forces minimal viable everything.** You only implement what's needed to evaluate your own gates. No speculative features.
- **Recursive validation.** If Assay can validate its own development, it can validate anyone's.
- **Natural story for README / blog post.** "Assay was built using Assay" is a compelling narrative.

**Scope:** 2-3 weeks. Spec format, gate evaluation (both tracks), CLI `check` command, enough infrastructure to self-host.

**Risks:**
- Circular dependency: you need the tool to validate the tool's development
- Bootstrap problem: how do you validate before the validator exists?
- May over-fit to Assay's own development needs (Rust-specific, single developer)
- Dogfooding can blind you to the needs of different projects/teams
- The spec format designed for self-use may not generalize

**Kills:** "Build features, then use them" assumption. "Development process is separate from the product" assumption.

---

## Proposal 6: Schema-Driven Everything (Types ARE the Product)

**What:** The first milestone produces NO working binary. Instead, it produces exhaustive JSON Schemas for every Assay concept: specs, gates, gate results, reviews, workflows, config, MCP tool inputs/outputs. The schemas are generated from `assay-types` via schemars and published to `schemas/`. Everything else — CLI, TUI, MCP server, plugins — is generated or hand-written against these schemas.

The thesis: in an agentic world, **the schema IS the product**. Agents don't use CLIs — they consume structured data. If the schemas are right, the implementations are interchangeable. Ship perfect schemas, then ship any implementation.

**Why this might be better:**
- **Agents consume schemas, not binaries.** An agent with the Assay JSON Schema can generate valid specs, gates, and workflows without any Assay binary installed.
- **Parallel implementation.** Once schemas are locked, CLI, TUI, MCP server, and plugins can all be built in parallel by different agents/developers.
- **Schema-first prevents API drift.** When the schema is the source of truth, implementations stay consistent.
- **Leverages what already exists.** `assay-types` already has serde + schemars derives. The schema pipeline was identified as a quick win. This proposal says: make it the ENTIRE first milestone.
- **Validates the type system early.** Getting types wrong means rewriting everything. Getting types right means everything else is mechanical.

**Scope:** 1-2 weeks. Redesign `assay-types` with complete domain model, generate JSON Schemas, write schema documentation, validate schemas against example data.

**Risks:**
- Schemas without working software are speculation crystallized into JSON
- No way to validate that schemas are correct without implementations
- Users can't DO anything with schemas alone
- Schema-first design often produces over-engineered, kitchen-sink types
- schemars-generated schemas may not be ergonomic for external consumers

**Kills:** "Ship working software first" assumption (contradicts "design for extraction" principle from previous brainstorm). "Types are infrastructure, not product" assumption.

---

## Summary Matrix

| # | Proposal | Core Idea | Speed to "Aha" | Engineering Risk | Diff. Proven? |
|---|----------|-----------|-----------------|------------------|---------------|
| 1 | Plugin-First | Build the Claude Code plugin, extract Rust later | 2-3 days | Low | Yes |
| 2 | Gate Demo | Ship ONLY dual-track gates + `assay check` | 1-2 weeks | Medium | Yes |
| 3 | MCP Server First | Agent-native interface before CLI | 2-3 weeks | Medium-High | Partial |
| 4 | Orchestrator Shell | Top-down with stubs | 2-3 weeks | High | Partial |
| 5 | Dogfooding Spec | Assay validates its own M1 | 2-3 weeks | Medium | Yes |
| 6 | Schema-Driven | Perfect types, no binary | 1-2 weeks | Low | No |

**My top picks for debate:** Proposals 1 (Plugin-First) and 2 (Gate Demo) are the most radical departures that still prove the differentiator. Proposal 5 (Dogfooding) is the most philosophically compelling. Proposal 4 (Orchestrator Shell) is the riskiest but potentially highest-payoff.
