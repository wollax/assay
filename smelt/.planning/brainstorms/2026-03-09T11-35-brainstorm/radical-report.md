# Radical Directions for Smelt — Final Report

**Explorer:** explorer-radical | **Challenger:** challenger-radical
**Date:** 2026-03-09 | **Debate rounds:** 2

---

## Executive Summary

Seven paradigm-shifting proposals were generated and pressure-tested through structured debate. Two were killed, two were deferred, and three emerged as a coherent capability ladder — each building on the last to create compounding competitive advantage.

**The core insight:** Smelt's radical differentiator isn't any single feature. It's the accumulation of *operational memory* — memory of the codebase (Digital Twin), memory of agent work (execution history), learning from that memory (Adaptive Workflows), and acting on that learning (Graduated Stewardship). This creates a data moat that deepens with every workflow run and cannot be bootstrapped by competitors.

---

## Approved Proposals (Ranked)

### 1. Codebase Digital Twin with Execution History

**What:** An incrementally-updated semantic model of managed codebases — AST/dependency graph via tree-sitter + graph store — that agents query instead of reading files. Critically, the twin also captures *workflow execution history*: which files agents modified together, which changes triggered test failures, which patterns led to PR approval.

**Why this wins:**
- Eliminates the #1 cost in agentic coding: context acquisition. Agents query the twin instead of burning tokens reading files.
- Execution history is a proprietary data advantage — Smelt gets smarter with every run. No competitor can bootstrap this data.
- Enables downstream capabilities (Adaptive Workflows, Stewardship) that are impossible without it.
- Axon has nothing comparable. Immediate differentiator.

**Seed version (3 months):** Tree-sitter AST parsing + dependency graph in a lightweight graph store. Agents query "what depends on this function?" and "what files are related to this module?" Execution history logged per-run, queryable via simple API.

**Constraints from debate:**
- Keep the twin project-scoped, not global. Execution history from Project A doesn't transfer to Project B.
- Skip "change impact simulation" for v1 — it requires near-whole-program analysis and is premature.
- Staleness is worse than absence. The incremental update pipeline must be rock-solid before agents trust the twin.

**Progression:** Seed → richer semantic analysis (type relationships, API surfaces) → execution pattern mining → cross-file change prediction.

---

### 2. Graduated Autonomous Stewardship

**What:** Smelt as a persistent, autonomous member of the engineering team that continuously monitors and maintains codebases. Ships as a suggestion dashboard; graduates to autonomous action through a trust-building progression:

- **Level 0 — Suggestions:** Continuous analysis surfaces maintenance tasks (dependency updates, dead code, test coverage gaps, CVEs) with confidence scores. Humans approve and trigger execution.
- **Level 1 — Auto-execute + PR:** Proven-safe categories auto-execute but require human PR review. (Dependency patch bumps, formatting fixes, documentation updates.)
- **Level 2 — Auto-merge:** After N successful human-approved runs of the same pattern category with zero reverts, auto-execute + auto-merge within defined boundaries.
- **Level 3 — Full autonomy:** Self-directed maintenance within explicit guardrails for teams that opt in.

**Why this wins:**
- This is the product vision that makes Smelt more than "run agents in Docker." It's the pitch: "Smelt handles your maintenance burden so your team focuses on product work."
- The Dependabot precedent proves graduated autonomy earns trust. Start safe, let the system's track record justify expanding autonomy.
- Code changes are reviewable and reversible — unlike self-driving cars, a bad commit is a `git revert`, not a fatality.

**Seed version (3 months):** Level 0 dashboard. Continuous codebase analysis, surfaced suggestions with confidence scores, one-click workflow execution for approved tasks.

**Constraints from debate:**
- Trust categories must be **explicit and user-defined**, not inferred. A major version bump (React 18→19) is a fundamentally different risk category than a patch bump, even though both are "dependency updates."
- Level 2+ requires **automated rollback infrastructure** as a prerequisite. Auto-merge without auto-revert is a bot that merges breakage.
- Design the architecture for Level 2+ from day one, even though v1 ships at Level 0.

**Progression:** Dashboard → auto-execute with PR → auto-merge for proven-safe → full autonomy within guardrails. Each transition gated by the system's own track record.

---

### 3. Protocol-Shaped Internal Architecture

**What:** Every internal agent interface in Smelt — capability declaration, task input/output contracts, context transfer format, lifecycle events — designed as if it will become an open protocol. Not marketed as a standard. Not published as a spec. Just clean, well-defined contracts that happen to be protocol-grade.

**Why this wins:**
- The LSP precedent: Microsoft designed LSP before VS Code was dominant, and the protocol *became* the growth strategy. The agentic orchestration market is fragmenting now — whoever defines the interop layer wins.
- Costs almost nothing extra. Protocol-shaped interfaces are just good architecture: explicit contracts, typed schemas, clear separation of concerns.
- Creates a 6-12 month head start. When the market matures enough for a standard, Smelt has battle-tested interfaces ready to publish.

**Seed version (immediate):** Design the agent adapter interface with capability schemas (what can this agent do?), typed task contracts (input/output schemas per step), context transfer format (how agents share codebase understanding), and lifecycle events (started, progress, blocked, completed, failed).

**Constraints from debate:**
- **Protocol-shaped is a design principle, not a deliverable.** If making an interface "protocol-shaped" adds complexity to shipping a feature, ship the feature first.
- Revisit the "publish as standard" question in 12 months when interfaces are battle-tested.
- Don't let the protocol ambition wag the product dog.

**Progression:** Internal contracts → battle-tested through Smelt's own multi-agent usage → evaluate market readiness → publish as open spec if/when timing is right.

---

## Honorable Mentions (Roadmap Items)

### Adaptive Workflows (natural extension of Digital Twin)

Smelt learns from workflow execution history to suggest workflow improvements. "In 8/10 runs that skipped linting, the PR was rejected — consider adding a lint step." Not a black-box optimizer — a transparent system that shows its reasoning and lets humans judge causation.

**Key constraint:** Must explain reasoning behind every suggestion. Transparent recommendations > opaque optimization. Guard against survivorship bias (encoding "whatever happened to work" as best practice).

### Temporal Debugging (natural extension of Observability)

Workflow checkpointing + replay. Save state at key decision points. Let users rewind and re-run from checkpoints with different parameters. The time-travel debugging value without the speculative execution cost.

**Descoped:** Full speculative execution (running N parallel branches) killed on economics — 2-10x cost multiplier doesn't fit the local-first, Docker-first user profile.

---

## Killed Proposals

| Proposal | Reason |
|---|---|
| **Agent Mesh (P2P)** | Coordination token overhead likely exceeds work cost. Agent self-assessment of capabilities is unreliable. The useful kernel (capability-aware dispatch) is folded into Digital Twin. |
| **Workflow Genetics** | No viable fitness function for workflows. Cross-org sharing has fatal IP/security concerns. Genetic algorithms converge on local optima in this domain. |

## Deferred Proposals

| Proposal | Defer Until | Reason |
|---|---|---|
| **Self-Evolving Workflows** | After Adaptive Workflows proves viable | The evaluation problem is solvable for *suggestions* but not for *autonomous workflow generation*. Adaptive Workflows is the stepping stone. |
| **Smelting Protocol (public standard)** | 12+ months | The interface boundary between Agent ↔ Orchestrator isn't stable enough to standardize. Build the dominant product first; extract the standard from battle-tested interfaces. |

---

## The Narrative Thread

These proposals aren't four separate features — they're one coherent capability ladder:

```
Digital Twin          → Agents gain memory of the codebase
  ↓
Execution History     → Agents gain memory of their own work
  ↓
Adaptive Workflows    → Agents learn from that memory
  ↓
Graduated Stewardship → Agents act on that learning
```

Each layer builds on the last. Each creates data that feeds the next. The compound effect is a system that gets meaningfully better with every workflow run — a flywheel that competitors cannot replicate by copying features, because the value is in the accumulated operational data.

**This is Smelt's radical thesis:** The future of agentic orchestration isn't smarter agents or better runtimes. It's *accumulated operational intelligence* — a system that has seen your codebase evolve, learned which agent strategies work for your project, and earned enough trust to act autonomously. That system is Smelt.
