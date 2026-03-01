# Radical Ideas: Final Consolidated Report

> Explorer: explorer-radical | Challenger: challenger-radical | Date: 2026-02-28
> Rounds of debate: 3

---

## Executive Summary

Six radical alternatives to the conventional bottom-up milestone sequencing were proposed and pressure-tested through adversarial debate. Four were killed or deferred. Two produced genuine structural insights that survived:

1. **Prove the differentiator first** — resequence the conventional plan to ship dual-track gates before config, persistence, or spec engine. Gates are the category-defining feature; they should be M1, not M4.
2. **Plugin as parallel research instrument** — run a quick Claude Code plugin prototype alongside the Rust build to discover agent interaction patterns that can't be learned from type design alone.

The debate also produced a critical honest scoping: agent invocation is the hardest unsolved problem and should be treated as an experimental track with a clean data contract but ugly transport, not blocked on perfect architecture.

---

## What We Proposed (6 Ideas)

| # | Proposal | Core Idea | Outcome |
|---|----------|-----------|---------|
| 1 | Plugin-First / Outside-In | Ship Claude Code plugin first, extract Rust later | **Reframed** → parallel research instrument |
| 2 | Dual-Track Gate Demo | Ship ONLY `assay check` with dual-track gates | **Adopted** → primary M1 deliverable (revised) |
| 3 | MCP Server First | Agent-native interface before CLI | **Killed** → transport ≠ product |
| 4 | Top-Down Orchestrator Shell | Build lifecycle with stubs | **Deferred** → inverted difficulty ordering |
| 5 | Dogfooding Spec | Assay validates its own M1 | **Deferred to M2** → circular dependency |
| 6 | Schema-Driven Everything | Perfect types, no binary | **Killed** → contradicts design-for-extraction |

---

## What We Killed and Why

### MCP Server First (#3) — KILLED

**Explorer's argument:** Agents are the primary users, so build the agent-facing interface first. The previous brainstorm already moved MCP from position 6 to position 3 — why not position 1?

**Challenger's critique that killed it:** MCP is transport, not product. `assay/check_gate` still needs gate evaluation logic. `assay/read_spec` still needs spec parsing. You haven't eliminated foundation work — you've wrapped it in a harder-to-debug transport layer. The CLI is faster for validating domain logic; MCP is a thin wrapper added later.

**Why it stays dead:** The observation that "agents are primary users" is correct but doesn't mean the agent transport layer should be built first. Build the domain logic, expose it via CLI for easy debugging, then wrap in MCP. The domain logic is identical regardless of transport.

### Top-Down Orchestrator Shell (#4) — DEFERRED

**Explorer's argument:** Build the full spec→work→gate→review→merge lifecycle with stub components. Integration bugs surface early. Fill in stubs empirically.

**Challenger's critique that deferred it:** The "build the chassis" analogy is misleading. The orchestrator (tmux management, worktree lifecycle, agent signaling, process cleanup) is the HARDEST part, not the chassis. This proposal builds the hardest thing first with stubs for the easy things. Hidden dependencies are massive: tmux not installed, agent crash recovery, dirty worktree state, OS-specific behavior. Orchestrator design without real components is inherently speculative.

**Why it's deferred, not killed:** The insight about integration bugs is real. But orchestration is M3-M4 work, after real components exist to orchestrate.

### Schema-Driven Everything (#6) — KILLED

**Explorer's argument:** Ship perfect JSON Schemas, no binary. Agents consume schemas. Enable parallel implementation.

**Challenger's critique that killed it:** Directly contradicts the previous brainstorm's strongest conclusion — "design for extraction, not standardization." Schemas without implementations are "speculation crystallized into JSON." Types are validated by usage, not by staring at JSON Schema. The schema generation pipeline is already a 1-hour quick win; elevating it to a full milestone adds delay without validation.

**Explorer's concession:** This was a reach. The contradiction with design-for-extraction is fatal.

### Dogfooding Spec (#5) — DEFERRED TO M2

**Explorer's argument:** Write M1's definition as an Assay spec. Implement just enough to evaluate its own gates. Recursive validation. "Assay was built using Assay."

**Challenger's critique that deferred it:** The circular dependency is fatal for M1. Change spec format → invalidate gates → rebuild evaluation → repeat. The first user of a pre-alpha tool is the worst user for validation (you work around bugs mentally). The narrative is equally compelling if done in M2 after the tool works.

**Explorer's concession:** Circular dependency is the central flaw, not a footnote. M2 dogfooding preserves the narrative without the engineering chaos.

---

## What Survived and How

### Insight 1: Prove the Differentiator First (from Proposal #2)

**The conventional plan sequences:** Error types → config → schemas → spec validation → gate dispatch. Gates don't ship until position 5.

**The radical resequencing:** Error types → gate types → deterministic gate evaluation → `assay check`. Gates ship as M1. Config, persistence, project init, schema gen — all deferred.

**Why this survived debate:**

- Explorer's argument: Dual-track gates are the category-defining feature. If they're what makes Assay different, prove they work before building plumbing.
- Challenger validated: "prove the differentiator first" is smart product thinking. But demanded honest scoping — the agent-evaluated track hides the hardest engineering problem.
- Compromise: Ship deterministic gates as the primary M1 deliverable. Add agent-evaluated gates as an experimental track (see Insight 3 below).

**Key debate refinement — gates need spec awareness:**

The challenger caught that a gate without spec context is "just a linter." The explorer countered that the gate FORMAT itself (structured natural-language criteria with context pointers and confidence levels) is independently novel. Resolution: gates carry a `SpecRef` (id + optional version) — not a full parsed spec, but enough to establish that gates are spec-aware. The output reads "Gate X for spec Y: PASSED" rather than "Gate X: PASSED." This costs 30 minutes and preserves the spec narrative.

### Insight 2: Plugin as Parallel Research Instrument (from Proposal #1)

**Original proposal:** Ship the Claude Code plugin FIRST, extract Rust later.

**Challenger's fatal critique:** "Extract later" is a rewrite, not an extraction. Plugin skills (markdown + YAML) share zero code with Rust domain logic. Plugin patterns teach UX but not implementation.

**Explorer's reframe that survived:** The plugin isn't a precursor — it's a research instrument. The critical unknowns for Assay aren't type design (Rust devs solve that easily). They're agent interaction patterns:

- Does the agent read the spec proactively or wait to be told?
- What's the natural interaction boundary? Does the agent call Assay or does Assay call the agent?
- Which planned features matter in practice and which are theater?

These are only learnable by experiencing the workflow from the agent's perspective. A weekend plugin prototype running alongside the Rust build isn't throwaway code — it's a structured experiment.

**Challenger's acceptance condition:** The research is only valuable if it produces a concrete deliverable — a "Plugin Research Findings" document listing specific type/interface changes recommended based on agent interaction experience. Without that artifact, it's a fun side project, not research.

**Final position:** Run a 2-3 day Claude Code plugin prototype in parallel with Track A. Required output: written findings document with specific recommendations that feed back into Track A's type design before it solidifies.

### Insight 3: Honest Agent Invocation Scoping (emerged from debate)

**The elephant in every room:** Three of six proposals (#1, #2, #5) depended on "invoke an agent" and none scoped it honestly. The challenger correctly identified this as the hardest unsolved problem in the project.

**Explorer's counter:** The difficulty is overstated. `claude --print "evaluate this assertion"` works TODAY. It's ugly, coupled to one agent, and not the final architecture. But it proves the concept in an afternoon.

**Challenger's acceptance with caveats:**

1. **Output parsing is the real risk.** Agents don't reliably produce parseable JSON. Budget a day for defensive parsing (extract JSON from markdown, handle missing fields, retry on parse failure).
2. **Feature-flag it properly.** A build-time feature gate (`--features agent-eval`), not just a CLI flag. Users without `claude` shouldn't see confusing errors.
3. **Define the contract, not the transport.** `AgentEvalRequest` and `AgentEvalResponse` structs in `assay-types` define the clean data contract. The subprocess invocation is ugly transport that gets replaced. The contract survives.

**Final position:** Agent-evaluated gates are Track C — experimental, behind a feature flag, with a clean contract and ugly transport. This is the honest way to ship the differentiator without pretending agent invocation is solved.

---

## The Converged Milestone Structure

### Track A — Primary Rust Build (~1.5-2 weeks)

1. **Error types** — `AssayError` with `#[non_exhaustive]`, structured variants, thiserror derives (0.5 days)
2. **Core types** — `SpecRef { id, version }`, `GateKind` enum, `GateCriteria`, `GateResult` with evidence, `AgentEvalRequest`/`AgentEvalResponse` contract types (1-2 days)
3. **Deterministic gate evaluation** — run shell commands, check exit codes, capture stdout/stderr as evidence, collect duration/timestamp (2-3 days)
4. **`assay check gates.toml`** — CLI subcommand, structured JSON output, human-readable summary, proper error reporting (1-2 days)

### Track B — Plugin Research (~2-3 days, parallel with Track A)

- Quick Claude Code plugin prototype implementing spec-gate workflow
- Focus on experiencing agent interaction patterns
- **Required deliverable:** "Plugin Research Findings" document with specific type/interface recommendations for Track A

### Track C — Experimental Agent Gate (~3-5 days, after Track A)

- Agent-evaluated criteria via subprocess (`claude --print`)
- Behind `--features agent-eval` build-time feature flag
- Defensive JSON extraction from agent output (preamble stripping, markdown unwrapping, retry)
- `assay check --agent gates.toml` evaluates both tracks
- Clean data contract in `assay-types`, ugly transport that gets replaced

### What This Defers from the Conventional Plan

| Deferred Item | Why | When |
|---------------|-----|------|
| Config loading | `assay check` takes a file arg, no config system needed | M2 |
| Project init (`.assay/` directory) | No persistence needed for stateless gate evaluation | M2 |
| Schema generation pipeline | 1-hour quick win, do it when schemas are needed by consumers | M2 |
| Full spec engine (markdown + frontmatter) | Gates carry `SpecRef`, full spec parsing is M2 | M2 |
| Spec validation | Minimal `SpecRef` validation only; full spec validation in M2 | M2 |
| MCP server | Transport layer, added after domain logic is proven | M2-M3 |
| Workflow state machine | Depends on working gates + specs | M3 |
| Review system | Depends on working workflow | M3-M4 |

---

## The Risk We Flagged

**"Is 'just gates' enough to feel like a milestone?"**

The challenger raised this as the real non-technical risk. M1 ships `assay check` with deterministic gates and experimental agent evaluation. That's technically solid, but "run shell commands and optionally ask an AI if your code is good" is a tough elevator pitch.

**Resolution:** M1 is a FOUNDATION milestone, not a LAUNCH milestone. It proves the differentiator works internally. M2 (spec workflow + config + persistence) is when the compelling README gets written. If the team is aligned on that framing, "just gates" is exactly right.

The demo narrative for M1 is internal/technical:
> "Here's a TOML file with quality criteria. Some are shell commands (tests, lints). Some are natural-language assertions an AI evaluates. `assay check` runs both and gives you structured results. No other tool does this."

The demo narrative for M2 is external/compelling:
> "Write a spec for what you want built. Assay ensures agents build it right — running your tests AND evaluating your natural-language quality criteria before anything merges."

---

## Key Takeaways

1. **The radical track's biggest contribution is resequencing, not reimagining.** The conventional approach builds the right things in the wrong order for proving product value. Putting gates first instead of config-first is a structural change that survived rigorous challenge.

2. **Plugin research as a parallel track is a novel process innovation.** No conventional plan would suggest "build a throwaway plugin to learn agent UX while building the real product in Rust." This emerged from the radical track and survived the challenger's hardest pushback once reframed from "precursor" to "research instrument."

3. **Honest scoping of hard problems beats ambiguous optimism.** The agent invocation problem appeared in three proposals and was honestly scoped in none. The debate forced clean separation: clean contract (types that survive), ugly transport (subprocess that gets replaced), proper isolation (feature flag). This is better engineering than either ignoring the problem or blocking on perfect architecture.

4. **Adversarial debate produced genuine convergence.** Explorer started with 6 proposals spanning "2-3 days plugin" to "2-3 weeks orchestrator." Challenger killed 4, reframed 2, and proposed a hybrid. Explorer conceded where critiques were valid and pushed back where they weren't. The result is tighter, more honest, and more actionable than either perspective alone.

5. **The milestone framing matters.** M1 as "foundation" (internal proof) vs M2 as "launch" (external demo) resolves the tension between "ship the differentiator early" and "ship something people want to use." Both are right — they're just different milestones.
