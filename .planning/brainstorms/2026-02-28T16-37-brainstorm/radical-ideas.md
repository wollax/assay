# Radical Ideas: Paradigm Shifts for Assay

> Explorer: explorer-radical | Date: 2026-02-28

---

## 1. The Agentic Development Protocol (ADP)

**What:** Instead of being a tool that wraps agents, Assay becomes the **protocol** — an open standard defining how AI agents collaborate on software. Like LSP standardized editor-language communication, ADP standardizes agent-spec-gate-review communication. Any agent (Claude, Codex, Gemini, open-source models) that speaks ADP can participate in spec-driven development. Assay the tool becomes the reference implementation; the real product is the protocol.

**Why:** The agent landscape is fragmenting fast. Every coding agent has its own way of receiving tasks, reporting progress, and handling feedback. This creates lock-in and makes multi-agent workflows impossible to compose. A protocol play makes Assay the connective tissue of the entire agentic development ecosystem rather than competing as one more tool in a crowded market. Protocol creators capture disproportionate value (HTTP, LSP, MCP).

**Scope:** Large. Requires formal specification work (likely a spec document, JSON-RPC or gRPC transport, reference implementations in multiple languages). Could start with a minimal viable protocol (spec submission, progress reporting, artifact delivery, review feedback) and expand. 3-6 months to initial spec, 6-12 months to ecosystem adoption.

**Risks:**
- Protocol adoption is brutally hard — network effects or nothing
- Premature standardization freezes bad abstractions
- Big players (Anthropic, OpenAI) may define their own protocols
- Community governance overhead could distract from building
- Risk of "architecture astronaut" syndrome — designing abstractions nobody uses

---

## 2. Executable Living Specifications

**What:** Specs aren't static markdown or YAML documents. They're **executable programs** that can verify their own fulfillment. A spec for "user authentication" doesn't just describe what auth should do — it contains runnable assertions that validate the implementation. Specs evolve alongside code: when an agent modifies behavior, the spec automatically queries whether intent is preserved. Think of it as "property-based testing meets requirements engineering" — specs define invariants that hold across all implementations.

**Why:** The fundamental problem with spec-driven development is spec drift — specs and code diverge over time. When AI agents write code at high velocity, drift accelerates. Executable specs close the loop: the spec IS the test, the test IS the spec. This eliminates the "write spec → implement → write tests → discover spec was ambiguous" cycle. It also enables something new: agents can interrogate specs programmatically to understand edge cases, constraints, and intent without human disambiguation.

**Scope:** Medium-large. Core requires a spec DSL or embedded language (could leverage Rust's macro system or a lightweight scripting engine like Rhai/Starlark). Need spec-to-assertion compilation, a runtime evaluator, and integration with existing test frameworks. 2-4 months for MVP.

**Risks:**
- Spec language design is hard — too simple is useless, too complex is unused
- Executable specs can become brittle if they over-specify implementation details
- Learning curve could deter adoption
- May duplicate existing property-testing/contract-testing tools
- Performance overhead of running specs as continuous assertions

---

## 3. Adversarial Multi-Agent Tribunal

**What:** Replace single-agent code review with an **adversarial tribunal system**. Three roles: the **Advocate** (argues why the change is correct and valuable), the **Prosecutor** (argues why the change is flawed, risky, or misaligned with intent), and the **Judge** (synthesizes arguments and renders a verdict with specific feedback). Each role is played by a different agent instance (potentially different models) with different system prompts optimized for their adversarial function. The tribunal produces a structured verdict: approve, reject, or remediate with specific requirements.

**Why:** Single-perspective code review has a ceiling — one agent's blind spots are systematic. Adversarial dynamics surface issues that consensus-seeking agents miss. This mirrors how legal systems, academic peer review, and red-team/blue-team security audits work. It also produces higher-quality feedback: instead of "looks good" or a list of nits, you get a reasoned argument for and against, with a synthesis. The structured output (verdict + rationale + remediations) feeds directly into automated spec-gate-review loops.

**Scope:** Medium. Requires agent orchestration (already in scope for Assay), prompt engineering for each tribunal role, a structured verdict schema, and integration with the review pipeline. The core orchestration logic is 1-2 months; prompt tuning is ongoing.

**Risks:**
- 3x API cost per review (three agent calls instead of one)
- Adversarial dynamics could produce artificial disagreements (theater, not insight)
- Latency: sequential tribunal deliberation could be slow
- Judge role quality depends heavily on prompt engineering
- Could feel over-engineered for simple changes (need a lightweight fast-path)

---

## 4. Intent Provenance Chain

**What:** Every artifact in the codebase carries a **provenance chain** linking it back to the originating intent. Code lines map to spec sections. Spec sections map to user stories or goals. When code changes, the chain is queried: "Does this change serve the original intent? Has intent shifted? Is this an intentional divergence or drift?" The chain is stored as structured metadata (not comments) that agents can query, traverse, and reason about. Think `git blame` but for *why* instead of *who*.

**Why:** As AI agents write more code, the connection between "what the human wanted" and "what the code does" becomes increasingly opaque. Agents don't have persistent intent — they follow instructions per-session. Without provenance, you end up with a codebase where nobody (human or agent) knows *why* something exists. Intent provenance enables: (1) automated impact analysis ("which specs are affected by this change?"), (2) intent-drift detection ("this code no longer serves its original purpose"), (3) audit trails for regulated industries, (4) onboarding for new agents/humans ("here's why this exists").

**Scope:** Medium. Core data model (intent graph linking specs → code → tests), storage layer (could be a sidecar DB or metadata files), query interface, and integration hooks for agents to annotate provenance during code generation. 2-3 months for core, ongoing refinement.

**Risks:**
- Provenance metadata can become stale if not actively maintained
- Granularity problem: line-level provenance is noisy, file-level is too coarse
- Storage overhead for large codebases
- Agents may not reliably self-report their intent during generation
- Could create false confidence ("the chain says this is fine" when it isn't)

---

## 5. Spec-as-Runtime: Continuous Specification Enforcement

**What:** Specifications don't retire after implementation. They become **runtime constraints** that are continuously enforced in production. A spec saying "API response time < 200ms" becomes a live SLO monitor. A spec saying "user data never leaves region X" becomes a runtime assertion in the data pipeline. Gates aren't just CI/CD checkpoints — they're deployed alongside the application as live sentinels. When a runtime spec violation occurs, it triggers the review process automatically, potentially spinning up an agent to diagnose and propose a fix.

**Why:** The current development model treats specs, tests, and monitoring as separate concerns with separate tooling. This creates gaps: code passes tests but violates specs in production; monitoring catches issues but can't trace them back to spec intent. Unifying specs across the lifecycle (design → build → test → deploy → operate) is the natural evolution of spec-driven development. It's especially powerful with agents: a spec violation in production can automatically trigger an agent to investigate, propose a fix, and submit it through the gate-review pipeline — closing the loop entirely.

**Scope:** Large. Requires a spec runtime (compiled specs as lightweight assertion agents), deployment integration (sidecar containers, middleware, or observability hooks), violation-to-agent triggering pipeline, and a production-safe execution model (violations observe and report, never block by default). 4-6 months for MVP.

**Risks:**
- Runtime overhead: specs executing in production consume resources
- Spec violations triggering automated agent fixes in production is terrifying (safety)
- Blurs development and operations in ways that may not be welcome
- Requires deep integration with deployment infrastructure
- Scope creep into observability/monitoring space (competing with Datadog, etc.)

---

## 6. Agent Capability Marketplace & Task Routing

**What:** Assay becomes an intelligent **task router** that matches work to agents based on capabilities, cost, speed, and quality track records. Agents register their capabilities (languages, frameworks, task types, quality metrics) with Assay. When a spec needs implementation, Assay evaluates registered agents, selects the best fit (or assembles a team), and routes work accordingly. Over time, Assay builds agent performance profiles: "Claude is excellent at Rust refactoring but mediocre at CSS; Codex is fast for boilerplate but misses edge cases." This becomes a marketplace where agent quality is measured, not assumed.

**Why:** The emerging reality is that no single agent is best at everything. Different models excel at different tasks, and their capabilities change with each update. Currently, developers manually decide which agent to use — a poor allocation of human attention. Automated capability-based routing maximizes quality and efficiency. The performance profiling aspect is especially valuable: it creates an empirical, per-project measure of agent quality that doesn't exist today. This data is enormously valuable — to developers choosing agents, to agent providers improving their models, and to Assay as a platform.

**Scope:** Medium-large. Requires agent registration protocol, capability schema, task analysis (decomposing specs into routable units), routing algorithm, performance tracking, and feedback loop. 3-5 months for core routing; ongoing for performance profiling.

**Risks:**
- Agent capabilities are hard to measure objectively and change frequently
- Routing latency adds to an already slow agent workflow
- Could create perverse incentives (agents gaming metrics)
- Multi-agent coordination complexity (who owns the final artifact?)
- Marketplace dynamics can be unpredictable and hard to balance

---

## 7. Temporal Specification Versioning & Parallel Agent Timelines

**What:** Specs exist on a **temporal axis**. Instead of a single current spec, you define a progression: v1 → v2 → v3, each building on the previous. Multiple agents can work on different temporal versions simultaneously — one agent implements v1 while another researches v3 feasibility. Quality gates enforce temporal consistency: v2 can't ship until v1's gates pass. This creates a natural pipeline where work flows forward in time, and you can see the entire planned evolution of a feature at a glance. It also enables "speculative execution" — agents can start on future versions before current ones are complete, with their work held in temporal branches.

**Why:** Software development is inherently temporal — features evolve through versions. But current tools treat each version as an independent task, losing the continuity of intent. Temporal specs make the evolution explicit and plannable. For agentic development, this is powerful: agents can parallelize across time, working on future versions speculatively while current versions are finalized. The temporal consistency gates prevent the chaos of premature optimization — you can't skip ahead. This also creates a natural "roadmap" that's not a separate document but is embedded in the specs themselves.

**Scope:** Medium. Core requires temporal versioning in the spec model, temporal dependency resolution in gates, parallel execution coordination, and temporal branch management. 2-4 months for core.

**Risks:**
- Complexity explosion: temporal versioning adds a dimension to every operation
- Speculative execution wastes resources if future specs change
- Temporal dependencies can create deadlocks if not carefully managed
- Mental model is complex for human users
- Merge conflicts across temporal branches could be nightmarish

---

## Summary Matrix

| Idea | Category | Impact | Feasibility | Risk |
|------|----------|--------|-------------|------|
| Agentic Dev Protocol | Platform/Ecosystem | Transformative | Hard | High |
| Executable Living Specs | Core Innovation | High | Medium | Medium |
| Adversarial Tribunal | Quality Innovation | High | Easy-Medium | Medium |
| Intent Provenance Chain | Core Innovation | High | Medium | Medium |
| Spec-as-Runtime | Paradigm Shift | Transformative | Hard | High |
| Agent Marketplace | Platform/Ecosystem | High | Medium-Hard | Medium-High |
| Temporal Spec Versioning | Workflow Innovation | Medium-High | Medium | Medium |
