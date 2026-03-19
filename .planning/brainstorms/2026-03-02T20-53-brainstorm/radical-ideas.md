# Radical New Directions for Assay

> Explorer: explorer-radical | Date: 2026-03-02
> v0.1.0 has shipped. These proposals challenge what Assay IS, not just what it does next.

---

## The Assumptions Being Challenged

v0.1.0 established Assay as a spec-driven quality gate system for AI coding agents. The current roadmap assumes:
1. Assay is a development tool for software projects
2. Gates verify code quality (tests, linting, file existence)
3. The unit of work is a spec → implement → gate → review cycle
4. One agent works on one spec at a time
5. Humans supervise agents through a TUI dashboard

What if some or all of these assumptions are wrong — or at least, unnecessarily limiting?

---

## Proposal 1: The Assay Protocol — Gates as a Universal Agent Accountability Layer

**Name:** "Agent Accountability Protocol"

**What:** Assay stops being a tool and becomes a **protocol**. Instead of being something you install into a project, Assay defines a standard for how ANY agent reports what it did, what evidence supports its work, and what quality bar it met. Think of it like OpenTelemetry for agent outcomes — not observability of HOW agents run, but accountability for WHAT agents produce.

The protocol defines:
- **Spec schema:** A universal format for declaring "what should be done" with acceptance criteria
- **Evidence schema:** A standard for structured proof that work was done (command outputs, file diffs, screenshots, test results)
- **Gate schema:** A composable format for quality checks (deterministic + agent-evaluated + human-approved)
- **Attestation schema:** A signed record that "agent X completed spec Y, passing gates Z, with evidence E"

Any tool, any agent, any CI system can produce and consume Assay attestations. The MCP server becomes one implementation. The CLI becomes another. But the protocol is the product.

**Why this could be transformative:**
- In 2026, the industry has dozens of AI coding agents but ZERO standards for agent accountability. There's no way to answer "did this agent actually do what it said?" with structured evidence. Assay could own this gap.
- Protocols create network effects. Once CI systems consume attestations, every agent tool has incentive to produce them. Once agents produce attestations, every review tool has incentive to consume them.
- It positions Assay above any individual agent. Not "works with Claude Code" but "works with any agent that speaks the protocol."
- The v0.1.0 types (Spec, GateResult, Criterion) are already halfway there — they're serializable, schema-backed, and agent-consumable.

**Scope:** Defining the protocol spec itself: 2-4 weeks. Reference implementation in Rust: 2-4 additional weeks. Getting adoption: months to years.

**Risks:**
- Protocols without adoption are just documentation. Getting buy-in from agent vendors is extremely hard.
- Premature standardization — the problem space is evolving so fast that any protocol might be wrong within 6 months.
- Standards bodies and competitors may ignore a solo developer's protocol.
- Could spread effort too thin: protocol design, reference implementation, evangelism, spec maintenance.
- Risk of designing for the abstract rather than solving real problems.

**The Bet:** Agent accountability will become a critical concern as AI-generated code enters production at scale, and no incumbent is positioned to own this category.

---

## Proposal 2: Assay Swarm — Multi-Agent Orchestration with Competitive Gates

**Name:** "The Swarm Forge"

**What:** Instead of one agent working on one spec, Assay orchestrates **multiple agents working on the same spec simultaneously**, then uses gates to select the best implementation. Think genetic algorithms applied to code: spawn N agents in N worktrees, let them all attempt the same spec, evaluate all of them against the same gates, and merge the winner.

But it goes further: agents can **compete on different strategies**. One agent might write a functional implementation, another OOP. One might prioritize performance, another readability. The gates evaluate each dimension and produce a ranked result. The human (or a meta-agent) selects which dimensions matter most.

The architecture:
- **Spawn pool:** Launch 2-5 agents against the same spec, each in an isolated worktree
- **Strategy hints:** Optional directives that bias each agent ("optimize for performance", "keep it simple", "maximize test coverage")
- **Parallel gate evaluation:** Run identical gates against all implementations simultaneously
- **Comparative analysis:** Produce a side-by-side comparison of all implementations with gate scores
- **Merge selection:** Human or automated selection of the winning implementation

**Why this could be transformative:**
- API costs are plummeting. Running 3 agents costs 3x tokens but produces dramatically better outcomes through selection pressure.
- This is the first agentic workflow that genuinely can't be replicated by a human developer — humans don't code the same thing three ways and pick the best.
- It solves the "agentic code quality" problem from a completely different angle: instead of trying to make one agent produce perfect code, let many agents compete and select the best.
- The gate system is already built for this — `gate_run` is stateless and can evaluate any worktree. The missing piece is orchestration, not evaluation.

**Scope:** Large. Worktree management, parallel agent spawning, comparative gate evaluation, selection UI. Probably 2-3 months for an MVP.

**Risks:**
- Cost multiplication: 3-5x token costs for marginal quality improvement may not be worth it for most tasks.
- Diminishing returns: agents with similar training may produce similar outputs, making competition meaningless.
- Merge complexity: even the "winner" may need manual integration into the main codebase.
- Orchestration complexity is enormous — managing N concurrent agent sessions with error handling, timeouts, and cleanup.
- The TUI/dashboard becomes essential (not optional) — you need visualization to compare N implementations.

**The Bet:** The marginal cost of running multiple agents will continue to drop faster than the marginal quality improvement of single-agent techniques, making competitive evaluation the dominant strategy.

---

## Proposal 3: Beyond Code — Assay as a Universal Spec-Gate Engine

**Name:** "Gates Everywhere"

**What:** The spec → gate → evidence loop isn't specific to code. It's a pattern that applies to ANY domain where work needs to be verified against criteria. Assay generalizes to become a **universal task verification engine** for agents working in any domain.

Consider:
- **Document writing:** Spec = "Write a blog post about X." Gates = word count check, readability score, fact-checking against sources, plagiarism detection.
- **Data analysis:** Spec = "Analyze sales trends for Q1." Gates = data validation, statistical significance checks, visualization quality, executive summary completeness.
- **Infrastructure:** Spec = "Deploy service X to staging." Gates = health check endpoints, performance benchmarks, security scan, rollback verification.
- **Design:** Spec = "Create a landing page mockup." Gates = accessibility audit, brand guideline compliance, responsive layout check, load time under 3s.

The key insight: Assay's `Criterion` already supports arbitrary shell commands. Any verification that can be expressed as a command that exits 0/1 is already a gate. The future `prompt` field (agent-evaluated gates) handles everything else. The system is already domain-agnostic — it just doesn't know it yet.

What changes:
- Spec authoring becomes domain-aware (templates for common verticals)
- Gate libraries become shareable (a marketplace of reusable criteria sets)
- Evidence types expand beyond stdout/stderr (screenshots, PDFs, metrics, API responses)
- The MCP server becomes the universal "did the agent do its job?" service

**Why this could be transformative:**
- The total addressable market expands from "developers using AI coding agents" to "anyone using AI agents for work." That's orders of magnitude larger.
- First-mover advantage in a category (agent work verification) that doesn't have a name yet.
- The current implementation is ALREADY domain-agnostic. `GateKind::Command` doesn't care if the command is `cargo test` or `curl -s https://api.com/health | jq .status`. Generalization is a framing shift, not a rewrite.
- Creates a flywheel: more domains → more gate libraries → more users → more domains.

**Scope:** Core engine changes are minimal (evidence types, spec templates). Creating domain-specific gate libraries is ongoing. The real effort is positioning, documentation, and go-to-market.

**Risks:**
- Jack-of-all-trades, master of none. Generalizing too early could mean doing nothing well.
- Code quality is the most validated use case. Abandoning that focus for nebulous "universal verification" could lose the existing niche.
- Each new domain requires deep domain expertise to create useful gates. A generic "word count" gate is useless without understanding what makes a good blog post.
- Competitors can build domain-specific tools faster than Assay can generalize.
- The agent ecosystem outside of coding is much less mature — there may not be enough demand yet.

**The Bet:** AI agents will rapidly expand beyond coding into every knowledge work domain, and task verification will be the universal bottleneck.

---

## Proposal 4: Assay as CI/CD for Agent Workflows — The Agent Pipeline

**Name:** "Agent Pipelines"

**What:** Assay becomes the **CI/CD system for agent-driven development**. Today, CI/CD (GitHub Actions, GitLab CI, Jenkins) assumes human developers commit code that triggers pipelines. But in an agentic world, the pipeline IS the development process. Assay defines pipelines where agents are the execution units, gates are the stage transitions, and specs are the pipeline definitions.

Think of it as GitHub Actions where the "jobs" are agent sessions:

```toml
# .assay/pipeline.toml
[[stages]]
name = "implement"
agent = { model = "claude-opus-4-6", prompt_from = "spec" }
gates = ["unit-tests", "lint", "type-check"]
on_fail = "retry"
max_retries = 3

[[stages]]
name = "review"
agent = { model = "claude-sonnet-4-6", role = "reviewer" }
gates = ["review-checklist", "security-scan"]
on_fail = "block"

[[stages]]
name = "integrate"
type = "merge"
strategy = "rebase"
gates = ["integration-tests", "e2e-tests"]
```

The pipeline handles:
- **Stage sequencing:** Implement → gate → review → gate → merge → gate
- **Agent lifecycle:** Spawn, monitor, timeout, retry, kill
- **Artifact passing:** One stage's output (code changes) is the next stage's input
- **Event triggers:** Git push, schedule, webhook, manual dispatch
- **Parallel fan-out:** Multiple specs processed concurrently with resource limits

**Why this could be transformative:**
- CI/CD is a $5B+ market that hasn't been rearchitected for the agent era. Every CI system assumes human-triggered pipelines. Assay can be the first agent-native CI.
- Developers already understand pipelines. Reframing Assay as "CI for agents" makes it instantly understandable — no need to explain "spec-driven development."
- The gate system maps perfectly to CI stage transitions. "All gates pass" = "stage succeeds" = "pipeline advances." This is isomorphic to existing CI concepts.
- Enterprise buyers already have CI/CD budgets. Positioning as "your CI/CD for AI agents" fits existing procurement categories.
- Composable with existing CI: an Assay pipeline could BE a GitHub Actions workflow step, or wrap around one.

**Scope:** Very large. Pipeline definition format, agent lifecycle management, event system, artifact management, retry logic, parallel execution, monitoring dashboard. This is essentially building a CI system. 6+ months for MVP.

**Risks:**
- Building a CI system is an enormous engineering effort. GitHub, GitLab, and CircleCI have teams of hundreds.
- Competing with GitHub Actions, which has massive distribution advantage (every GitHub repo), is suicidal on features.
- Agents are inherently non-deterministic. CI assumes deterministic stages. The impedance mismatch creates fundamental design challenges.
- Pipeline complexity could overwhelm solo developer / small team capacity.
- The market may not be ready — most teams are still experimenting with single-agent workflows.

**The Bet:** CI/CD will be completely reimagined around agent-driven workflows within 2 years, and the first credible "Agent CI" tool will capture outsized market share.

---

## Proposal 5: The Evidence Chain — Auditable Agent History

**Name:** "Provenance Ledger"

**What:** Every piece of agent-generated code gets a tamper-evident **provenance chain** — a cryptographically linked record of: which spec defined the work, which agent did it, what gates it passed, what evidence was collected, and who (human or agent) approved it. Think git commit history, but for the entire agent workflow, not just the final code changes.

This isn't blockchain (no distributed consensus needed). It's a Merkle-tree-style append-only log, stored alongside the git repository, that provides:
- **Traceability:** For any line of code, answer "which spec required this, which agent wrote it, and what evidence proved it was correct."
- **Auditability:** Compliance officers, security teams, and regulators can verify that quality processes were followed without reading code.
- **Reproducibility:** Replay the exact spec + gate configuration that produced a given result.
- **Accountability:** When agent-generated code breaks production, the evidence chain shows exactly what was checked and what was missed.

The implementation:
- Each gate evaluation produces a signed `GateResult` with a hash of inputs (spec, working directory state, timestamp)
- Results chain together: each result references the hash of the previous result
- The chain is stored in `.assay/evidence/` as append-only JSON files
- A `assay audit` command verifies the chain and produces compliance reports
- MCP tools expose `evidence_query` for agents to search the provenance history

**Why this could be transformative:**
- Regulatory pressure is coming. The EU AI Act, SOC 2 for AI, and industry-specific regulations will require evidence that AI-generated code meets quality standards. Assay can be the compliance tool.
- Enterprise adoption of AI coding agents is blocked by "how do we audit this?" Provenance chains answer that question.
- No existing tool provides structured, verifiable evidence of agent work quality. Git history shows WHAT changed but not WHY or HOW it was verified.
- The current `GateResult` type already captures most of what's needed (passed, evidence, timestamp, kind). Adding cryptographic chaining is incremental.
- Creates lock-in: once an organization's compliance process depends on Assay evidence chains, switching costs are very high.

**Scope:** Core evidence chain implementation: 3-4 weeks. Audit CLI: 1-2 weeks. Compliance report generation: 2-3 weeks. Getting regulatory recognition: long tail.

**Risks:**
- Premature optimization for a regulatory environment that may take years to materialize.
- Over-engineering cryptographic guarantees that no one actually needs or verifies.
- The overhead of chain maintenance (hashing, signing, storing) could slow down fast-moving development.
- If compliance requirements emerge differently than predicted, the chain format may be wrong.
- Solo developer building compliance tooling lacks credibility with enterprise buyers.

**The Bet:** AI code governance will become a regulated domain within 2-3 years, and organizations will pay premium prices for tools that provide auditable evidence of AI code quality.

---

## Summary Matrix

| # | Proposal | Core Shift | TAM Impact | Eng. Effort | Diff. Strength |
|---|----------|-----------|------------|-------------|----------------|
| 1 | Agent Accountability Protocol | Tool → Protocol/Standard | Massive | Medium | Very High |
| 2 | Swarm Forge | Single agent → Competitive multi-agent | Same market, better product | Large | High |
| 3 | Gates Everywhere | Code-only → Any domain | Massive | Medium (framing) | Medium |
| 4 | Agent Pipelines | Dev tool → CI/CD for agents | Large ($5B+ market) | Very Large | High |
| 5 | Provenance Ledger | Quality check → Compliance/audit trail | Large (enterprise) | Medium | Very High |

**My rankings by transformative potential:**
1. **Protocol (#1)** — category-defining if it gets adoption
2. **Provenance (#5)** — inevitable regulatory demand, first-mover advantage
3. **Gates Everywhere (#3)** — lowest effort, highest TAM expansion
4. **Swarm Forge (#2)** — genuinely novel capability, but expensive
5. **Agent Pipelines (#4)** — biggest market but biggest engineering effort
