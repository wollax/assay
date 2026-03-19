# Radical Directions — Final Report

> Explorer: explorer-radical | Challenger: challenger-radical | Date: 2026-03-02
> 2 rounds of debate. Converged on all 5 proposals.

---

## Executive Summary

Five radical directions were proposed and pressure-tested. None warrant a pivot from the existing vision (orchestrator, merge-back, dual-track gates, TUI dashboard). Instead, the debate produced:

- **2 confirmed kills** that prevent future exploration waste
- **1 deferred feature** with architectural preservation notes
- **1 design principle** extracted from a non-proposal
- **1 well-scoped feature direction** (Gate History) that enhances the core vision
- **1 architectural recommendation** for the orchestrator

The existing vision is the right product. These outputs make it stronger.

---

## 1. KILLED: Agent Accountability Protocol

**Original proposal:** Assay becomes a protocol/standard for agent accountability — schemas for specs, evidence, attestations that any tool can produce and consume.

**Why killed:** This was already rejected in the Feb 28 brainstorm with the rationale "design for extraction, not standardization." Every successful protocol (LSP, MCP, HTTP) was extracted from working software, never designed in advance. Assay has 5,028 lines of code and zero external users. Standardizing types that haven't been validated by real-world usage is premature. Protocol adoption requires a network effect that a solo developer with a zero-user tool cannot create.

**Salvage:** The observation is correct — agent accountability is an emerging need. The types are already serializable with JSON Schema generation via schemars. That's sufficient infrastructure for future protocol extraction IF adoption materializes. No work needed.

**Agreed unanimously in Round 1.**

---

## 2. DEFERRED: Swarm Forge (Competitive Multi-Agent)

**Original proposal:** Orchestrate N agents against the same spec in isolated worktrees, evaluate all implementations with gates, select the best.

**Why deferred (not killed):**
- The insight is genuinely novel — competitive evaluation with strategy hints ("optimize for performance" vs "maximize readability") produces structured variation that gates can compare.
- But it requires single-agent orchestration first (unbuilt), dual-track gates with agent evaluation (unbuilt), and competitive evaluation infrastructure (massive scope).
- Token cost economics are improving but don't yet justify 3-5x multiplication for most tasks.

**Architectural preservation:** The current design already supports this future:
- Gate evaluation is stateless: `evaluate(criterion, working_dir, timeout)` accepts any path. No single-agent assumption.
- Worktree management (when built) should be generic, not hardcoded to one worktree per spec.
- No code changes needed — the architecture is already Swarm-compatible.

**Revisit condition:** When single-agent orchestration works AND token costs drop another 50-70% AND agent-evaluated gates ship. Estimated: 2027+.

**Agreed after Round 2 pushback on strategy hints and cost trends.**

---

## 3. ABSORBED AS DESIGN PRINCIPLE: Gates Everywhere

**Original proposal:** Generalize Assay beyond code to any domain — document writing, data analysis, infrastructure, design.

**Why it's not a proposal:** The architecture is already domain-agnostic. `GateKind::Command` doesn't care if the command is `cargo test` or `curl -s health.json`. `GateResult` captures generic evidence (stdout, stderr, exit_code). There is nothing to build.

**Design principle extracted:**
> **Keep core types domain-agnostic.** Never introduce code-specific types like `CodeQualityGate` or `TestResult` into `assay-types` or `assay-core`. Gate evaluation, evidence capture, and spec definitions should remain generic. Position for software development; architect for any domain.

This is already true in v0.1.0. The principle prevents accidental narrowing in future work.

**Agreed unanimously in Round 1. Challenger correctly identified: "this is an insight, not a proposal."**

---

## 4. KILLED: Agent Pipelines (CI/CD for Agents)

**Original proposal:** Assay becomes a CI/CD system where agents are execution units, gates are stage transitions, and specs are pipeline definitions.

**Why killed:**
- Building a CI system is a years-long, team-scale engineering effort. GitHub Actions had dozens of engineers. A solo developer cannot compete.
- Existing CI systems (GitHub Actions, GitLab CI) will add agent-aware primitives. They have distribution, teams, and budgets. Assay's competitive advantage is zero.
- The scope explosion (pipeline definitions, event systems, artifact management, retry logic, parallel execution, monitoring) replaces the product with a different product entirely.
- CI assumes deterministic stages. Agents are non-deterministic. The impedance mismatch is fundamental.

**Architectural recommendation extracted:**
> **Design the orchestrator with pipeline semantics internally.** The orchestrator IS a single-spec pipeline: `implement → gate → review → merge` with conditional transitions. Use explicit stage/transition/condition abstractions:
> - A `Stage` concept (Implement, Gate, Review, Merge)
> - Transition conditions (gate passes → advance; gate fails → retry/block/notify)
> - Per-stage configuration (timeout, retry count, agent selection)
>
> This is good domain modeling, not CI system building. It makes the orchestrator's internal model cleaner and more extensible. Future CI integration (e.g., GitHub Actions wrapping Assay stages) becomes trivial.

**Kill agreed in Round 1. Architectural salvage agreed in Round 2.**

---

## 5. ADOPTED AS FEATURE DIRECTION: Gate History

**Original proposal:** "Provenance Ledger" — tamper-evident cryptographic evidence chains for agent-generated code, targeting regulatory compliance.

**What survived the debate:** The cryptographic and compliance angles were correctly challenged as premature. But the core insight — persisting gate results over time — enables high-value developer workflow features:

| Use Case | Description | Phase |
|----------|-------------|-------|
| Gate result diffing | "Passed 5/5 yesterday, 4/5 today — what changed?" | 2 |
| Trend analysis | "This criterion passes 70% of the time — it's flaky" | 2 |
| Agent evaluation context | Prior gate results inform agent-evaluated criteria | 2 |
| Regression detection | "Gate X took 200ms, now takes 8s — performance regression in gates" | 2 |
| Compliance/audit trail | Signed evidence chains for regulatory requirements | 3 |

These are developer workflow features first, compliance features second. The regulatory angle is icing, not cake.

**Renamed:** "Provenance Ledger" → **"Gate History"** — names what it actually is without overselling cryptographic guarantees that should wait.

**Phased scope:**

- **Phase 1 (v0.2):** Persist `GateResult` records to `.assay/evidence/<spec>/<timestamp>.json` after every `gate run`. Simple append-only file writes. Design the file layout and indexing thoughtfully — Phase 2 use cases (diffing, trends, querying) place real constraints on the data model. ~2-3 days.
- **Phase 2 (v0.3+):** Query interface, comparison between runs, trend visualization. Scope determined by what Phase 1 users actually want. ~1-2 weeks.
- **Phase 3 (future):** Cryptographic chaining, signing, compliance reports. Only if demand materializes from real users.

Each phase is independently valuable. Phase 1 fits naturally into v0.2 gate improvements.

**Convergence path:** Explorer initially proposed 8-week scope. Challenger proposed 1-2 day afterthought. Final agreement: 2-3 day designed feature (Phase 1) with phased expansion. The design decisions matter (file layout, indexing, query interface) but the cryptographic infrastructure does not — yet.

---

## Summary of Outputs

### For the Roadmap

| Output | Type | Effort | When |
|--------|------|--------|------|
| Gate History Phase 1 (persistence) | Feature | 2-3 days | v0.2 |
| Pipeline semantics in orchestrator | Architecture | Design-time | When orchestrator is built |

### Design Principles (Ongoing)

1. **Keep core types domain-agnostic.** No code-specific types in `assay-types` or `assay-core`.
2. **Keep gate evaluation stateless.** Accept arbitrary paths; never assume single-agent-per-spec.
3. **Design for extraction, not standardization.** Protocol work is a post-adoption exercise.

### Confirmed Kills (Do Not Revisit)

1. **Agent Accountability Protocol** — premature standardization, recycled rejected idea
2. **Agent Pipelines / CI system** — scope explosion, unwinnable competition with incumbents

### Deferred (Revisit When Prerequisites Met)

1. **Swarm Forge / Competitive Multi-Agent** — revisit when single-agent orchestration + agent-evaluated gates ship AND token costs drop further
