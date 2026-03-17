# Radical Directions Report: Assay v0.3.0+

**Author:** Team lead (radical pair was stuck; written directly)
**Date:** 2026-03-08
**Status:** Self-challenged — each proposal includes its own counter-arguments

---

## Executive Summary

Three paradigm-shifting ideas explored for Assay's future direction. These are NOT v0.3.0 scope — they're strategic options that inform how v0.3.0 features should be designed to keep doors open. One idea has a viable v0.3.0 seed.

---

## Proposals

### 1. Gate Marketplace — Shareable Quality Gate Definitions

**What:** A registry/marketplace where teams share gate definitions (criteria sets, evaluation prompts, threshold configs). Think "ESLint shared configs" but for AI quality gates. Users install gate packs: `assay gate install @company/rust-standards`.

**Why this could be transformative:** Quality gates are only as good as the criteria. Most teams will write mediocre criteria. A marketplace lets expert-crafted criteria propagate. This is how ESLint, Prettier, and cargo-deny became ubiquitous — shared configs that encode community expertise.

**Core insight:** Assay's gate definitions are already serializable TOML. The infrastructure for sharing is almost free — it's a naming convention + resolution strategy.

**If it works:** Assay becomes the distribution channel for quality standards, not just the enforcement engine. Network effects kick in.

**Risks:**
- Premature without a user base — who contributes to a marketplace with 1 user?
- Gate criteria are highly project-specific — shared configs may not generalize well
- Versioning and compatibility become complex fast

**MVP to prove the concept:** Add a `[gate.extends]` field that imports criteria from a local path or git URL. No registry, no marketplace — just composable gate definitions. This is a ~2 day feature that opens the door without committing to infrastructure.

**v0.3.0 relevance:** If Gate Evaluate (high-value #4) defines evaluation configs in TOML, design them to be composable from the start.

---

### 2. Trust Scores — Quantified Agent Reliability

**What:** Track agent quality over time across runs. Each agent builds a trust score based on: gate pass rates, evaluation agreement (self vs independent), criteria-specific reliability. Surface this as `assay trust show` — "Claude Code passes 87% of deterministic gates, 62% of independent evaluations on auth-related criteria."

**Why this could be transformative:** The fundamental problem with AI coding agents isn't capability — it's predictability. You don't know which tasks an agent will nail and which it'll botch. Trust scores turn this from a feeling into data. Over time, Assay could route work to agents based on demonstrated competence: "Agent A is good at tests, Agent B is good at refactoring."

**Core insight:** Assay already collects all the raw data — gate results, run history, enforcement outcomes. Trust scores are an aggregation layer on top of existing persistence.

**If it works:** Assay becomes the authority on agent quality, not just the enforcement point. This is a unique position no other tool occupies.

**Risks:**
- Statistical validity — meaningful scores need hundreds of runs per agent
- Gaming — agents optimizing for scores rather than actual quality
- Overfitting — trust in one domain doesn't transfer to another
- Scope creep — easy to build a data science project instead of a dev tool

**MVP to prove the concept:** Add `assay gate history --summary` that shows pass/fail rates per spec, per gate kind (command vs agent), over time. Pure aggregation of existing data, ~1 day. If the numbers tell a useful story, invest in the full trust model.

**v0.3.0 relevance:** Run history already captures `trigger` (CLI/MCP/Agent) and enforcement outcomes. Ensure Gate Evaluate results are tagged with evaluator identity so trust can be computed per-evaluator later.

---

### 3. Spec-as-Contract — Bidirectional Quality Agreements

**What:** Flip the spec model from "instructions for the agent" to "contract between human and agent." The spec defines not just what the agent must deliver, but what the human provides (context, constraints, review SLA). Gate results become evidence of contract fulfillment — both sides. If the spec says "implement auth flow" but doesn't provide the auth provider docs, the agent's failure is a spec failure, not an agent failure.

**Why this could be transformative:** Every current AI coding tool treats the agent as a subordinate executing orders. But the best human-AI collaboration is a negotiation. Specs-as-contracts formalize this: "I'll give you X context, you'll deliver Y quality, we'll both be held accountable."

**Core insight:** Assay already has the two sides — specs (human's commitments) and gates (agent's commitments). The gap is that specs don't validate the human's side. Adding "precondition gates" that verify the spec itself is well-formed and sufficient closes this loop.

**If it works:** Assay reframes AI coding from "agent does what it's told" to "human and agent collaborate with mutual accountability." This is a genuinely new paradigm.

**Risks:**
- Philosophical — most developers won't accept that they should be "gated" too
- Complexity — bidirectional contracts are exponentially harder to validate
- Adoption friction — adding requirements to spec authors reduces uptake

**MVP to prove the concept:** Add a `[preconditions]` section to specs with simple checks: "file exists," "dependency installed," "branch is clean." Gate evaluation fails fast with actionable messages if preconditions aren't met. This is ~1-2 days and immediately useful — it catches "you forgot to install the test framework" before the agent wastes 10 minutes.

**v0.3.0 relevance:** The preconditions concept fits naturally into the existing spec/gate model. If v0.3.0 adds `gate_evaluate` with context assembly, preconditions could validate that sufficient context exists before evaluation begins.

---

## Cross-Cutting Themes

1. **Assay's unique position is data.** Every radical direction leverages the same asset: structured quality data from gate runs. The marketplace shares gate definitions, trust scores aggregate gate results, contracts validate both sides of gate agreements. v0.3.0 should prioritize rich, well-tagged data capture.

2. **Composability is the unlock.** Gate marketplace = composable definitions. Trust scores = composable metrics. Contracts = composable agreements. Design v0.3.0 types to be composable (extend, inherit, reference) rather than monolithic.

3. **The agent-as-peer shift.** All three ideas treat the AI agent as a peer rather than a tool. This is where the industry is heading. Assay can lead this shift by building infrastructure that assumes collaboration rather than command.

---

## Recommended Seeds for v0.3.0

| Radical Idea | Seed Feature | Effort | Where It Fits |
|---|---|---|---|
| Gate Marketplace | `[gate.extends]` for composable criteria | 2 days | Spec type refinement |
| Trust Scores | `gate history --summary` with pass rates | 1 day | CLI enhancement |
| Spec-as-Contract | `[preconditions]` section in specs | 1-2 days | Spec type refinement |

Total: ~4-5 days of seed work that opens three strategic doors.

---

*Written by team lead after radical explorer/challenger pair timed out — 2026-03-08*
