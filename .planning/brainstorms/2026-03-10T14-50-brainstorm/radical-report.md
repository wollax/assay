# Radical Ideas: Final Report

> Debate rounds: self-critique incorporated (challenger-radical was blocked on task #5 completion)

---

## Executive Summary

Six radical proposals were generated and stress-tested against common failure modes for developer tooling. The unifying thesis — **gates are data, not just checks** — survived scrutiny, but several ideas required significant scoping adjustments to be actionable.

**Top 3 for v0.4.0 consideration**: Gate Oracle seed, Quality Archaeology (local), Fractal Gates.
**Deferred to v0.5.0+**: Causal Context Engine, Gate Mutation Testing, Adaptive Specs.

---

## Proposal 1: Gate Oracle (Predictive Failure Prevention)

### Original Claim
Before running gates, compute failure probability from co-failure history patterns. Transforms Assay from reporter to predictor.

### Critical Pressure
- **Cold-start problem is severe**: New projects have zero history. The feature is invisible to most first-time users and only valuable after 20+ runs. This creates a deferred value proposition that's hard to market.
- **Prediction liability**: A "likely to fail" warning that's wrong trains users to ignore predictions entirely. False confidence is worse than no prediction.
- **Frequency counting ≠ Oracle**: The name "Oracle" overpromises. Co-failure frequency is a weak signal — it doesn't distinguish "this always fails because the team is ignoring it" from "this is genuinely risky."

### Refined Proposal
**Rebrand to `gate_health`** — a per-criterion pass-rate report, not a prediction. Remove the forward-looking framing entirely in v0.4.0. Show: "This criterion has failed 4 of last 5 runs" as a diagnostic, not a prediction. Genuine prediction deferred to v1.0 if data supports it.

**v0.4.0 scope**: `gate_health` MCP tool. Shows pass rate, last N results, and flags criteria with >60% failure rate. Pure history aggregation, no prediction.

---

## Proposal 2: Fractal Gates (Spec Composition)

### Original Claim
Specs reference other specs via `uses:` field. Hierarchical quality contracts with enforcement overrides.

### Critical Pressure
- **DAG complexity**: Cycle detection, evaluation ordering, and failure attribution across a dependency graph are non-trivial to implement correctly and explain to users.
- **Duplication is intentional sometimes**: Copy-paste specs are explicit. Composition introduces implicit dependencies — if a shared base spec changes, all composing specs silently change. This is a footgun for teams.
- **Real need is simpler**: Most teams don't need composition; they need *criteria inheritance with override*. A `release` spec that inherits `ci` but promotes advisory criteria to required.

### Refined Proposal
**Start with simple criteria inheritance, not full composition.** A `strict_mode: true` field in a spec that promotes all advisory criteria to required. Or an `extends: ci-base` field that inherits criteria with optional overrides. No DAG in v0.4.0 — just single-level inheritance.

**v0.4.0 scope**: `extends:` field for single-level spec inheritance. Child spec can override per-criterion enforcement. No multi-level composition.

---

## Proposal 3: Causal Context Engine (Counterfactual Analysis)

### Original Claim
After a gate failure, identify what context was pruned that would have prevented it. Adaptive pruning based on gate outcomes.

### Critical Pressure
- **The causal claim is almost certainly wrong**: We'd have correlation between "context was present" and "gate passed," not causation. An agent that says "this file's absence caused the failure" could be hallucinating a plausible-sounding story.
- **Circular dependency**: To know what context would have helped, you need to re-run with full context — which defeats the purpose of pruning.
- **Premature optimization**: Context pruning is only a problem at scale. Most projects don't hit token limits on single gate evaluations.

### Refined Proposal
**Scope to diagnostic logging only.** When a gate fails, log: (a) whether context was pruned during this session, (b) what the pruning report contained, (c) context utilization % at time of failure. This gives humans the data to form their own causal hypotheses without Assay overclaiming.

**v0.4.0 scope**: Log pruning metadata alongside gate failure records in history. Add `pruning_context` field to `GateRunRecord`. Surface this in `gate_history` output.

---

## Proposal 4: Gate Mutation Testing (Gate Coverage Score)

### Original Claim
Introduce deliberate code mutations in worktrees, run gates, measure which gates catch mutations. Produce a Gate Coverage Score.

### Critical Pressure
- **Mutation generation needs agent calls**: Translating "criterion description" into targeted code mutations requires agent reasoning. This is expensive, slow, and unpredictable.
- **Scope mismatch**: Mutation testing evaluates whether your *code tests* catch mutations. Gate mutation testing conflates "does the gate command run?" with "does the gate command catch this specific bug?" These are different problems.
- **The actual value is narrower**: The real insight is "this gate always passes regardless of what I do." That's detectable without mutations — just run the gate against a blank project and see if it passes.
- **`cargo mutants` already exists**: For Rust projects specifically, this is a solved problem for code-level mutation. Assay shouldn't duplicate it.

### Revised Assessment
This is genuinely v0.5.0+ work, and possibly not Assay's domain. The more valuable and in-scope version: **Gate Sanity Checks** — verify that required gates can actually fail (not always-pass). Simple, deterministic, valuable.

**v0.4.0 scope**: A `gate_sanity` command that runs each gate against a deliberately broken state and flags any required gate that passes anyway. Uses worktree isolation. No agent, no mutation generation.

---

## Proposal 5: Quality Archaeology (History as Semantic Database)

### Original Claim
Index gate history into SQLite, enable trend queries, eventual federation.

### Critical Pressure
- **SQLite adds migration overhead**: Once you have SQLite, you own schema migrations forever. The current JSON-on-disk approach has zero migration cost.
- **Federation is a trust and privacy problem**: Sharing quality data requires a central server, authentication, anonymization guarantees, and legal review. This is product scope, not tooling scope.
- **Query complexity creep**: A natural-language query interface (`gate_query "show trends"`) requires an agent call for every history lookup. That's expensive for something that should be instant.

### Refined Proposal
**Keep JSON-on-disk, add in-memory aggregation.** Implement history aggregation in Rust (no SQLite) that computes: pass rate per criterion, trend over last N runs, worst-performing criteria. Cache results in a lightweight `.assay/history-cache.json`. Invalidate on new runs. No migration concerns.

Federation deferred indefinitely — it's a business/product decision, not a v0.4.0 engineering decision.

**v0.4.0 scope**: `gate_history` enhanced with `--summary` flag. Rust aggregation over existing JSON files. Shows: pass rate per criterion (all time + last 7 runs), trend indicator (↑↓→), worst 3 criteria.

---

## Proposal 6: Adaptive Specs (Self-Amending Quality Contracts)

### Original Claim
Specs that propose their own amendments based on observed behavior. Human-approved diffs.

### Critical Pressure
- **Gradually weakening standards**: Any mechanism that "suggests downgrading required to advisory" creates organizational pressure to accept those suggestions. Teams optimize for metrics — if the tool keeps suggesting downgrade, teams will accept it.
- **Correlation ≠ spec miscalibration**: A criterion failing 70% of the time might mean the criterion is too strict, OR it might mean the team has a persistent quality problem. The tool can't distinguish these — but the human reading the "suggestions" will assume the former.
- **PR-style diffs for spec changes is good UX, but wrong framing**: The value isn't "the spec proposes changes" — it's "the spec provides evidence for human decision-making."

### Revised Assessment
Drop the "self-amending" framing entirely. Instead: **Spec Health Report** that gives teams the data they need to make calibration decisions themselves. `gate_health` (from Proposal 1) already covers this.

**v0.4.0 scope**: Merged into `gate_health`. No separate feature needed.

---

## Consolidated v0.4.0 Recommendations

| Feature | Tool/Flag | Mechanism | Risk |
|---------|-----------|-----------|------|
| `gate_health` | MCP tool | History aggregation over JSON, no DB | Low |
| `extends:` spec inheritance | Spec TOML field | Single-level criteria inheritance | Medium |
| Pruning metadata in history | `GateRunRecord` field | Log alongside failures | Low |
| `gate_sanity` | CLI command | Worktree + broken-state check | Medium |
| History summary | `gate_history --summary` | In-memory Rust aggregation | Low |

## Deferred (v0.5.0+)

- True predictive failure modeling (needs 100+ run corpus)
- Full spec composition DAG
- Causal context analysis
- Agent-driven mutation testing
- Federation / community benchmarks

---

## What Survived Pressure Testing

**The core thesis holds**: Gates-as-data is the right direction. The mistake was overbuilding the intelligence layer before the data layer is mature. The refined v0.4.0 roadmap focuses on:

1. **Instrument everything** (pruning metadata, richer history)
2. **Make history queryable** (aggregation, not a new DB)
3. **Simplest useful composition** (single-level inheritance)
4. **Sanity, not intelligence** (gate_sanity over Gate Oracle)

The v1.0 vision (predictive, adaptive, federated) becomes achievable only because v0.4.0 builds the data foundation correctly.
