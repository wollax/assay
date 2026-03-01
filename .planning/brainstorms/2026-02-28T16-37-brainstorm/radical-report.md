# Radical Ideas: Final Consolidated Report

> Explorer: explorer-radical | Challenger: challenger-radical | Date: 2026-02-28
> Rounds of debate: 3

---

## Executive Summary

Seven radical proposals were explored and pressure-tested through adversarial debate. Two genuine paradigm-shifting insights survived:

1. **Dual-track executable specs** (deterministic + agent-evaluated criteria) — a novel combination nobody else offers
2. **Agents as the spec evaluation engine** — eliminates the need for a custom specification language entirely

The remaining five proposals were either scoped down to simpler, more practical versions or correctly deferred. The debate also produced a clear implementation sequencing and identified what NOT to build.

---

## The Core Thesis: What Makes Assay Different

Assay is the only open-source, terminal-first tool that combines:

1. **Executable specs** with dual-track criteria (deterministic + agent-evaluated)
2. **Convention-based intent provenance** on top of git
3. **Structured gated reviews** with optional two-pass bias checking
4. **Agent-agnostic design** via protocol-ready internal interfaces

No existing tool in the competitive landscape (Spec Kit, Kiro, Zenflow, BMAD, Plandex) offers this combination. Spec Kit provides markdown specs with no enforcement. Kiro enforces but is a closed IDE. None have the dual-track deterministic + agent-evaluated approach.

---

## Proposals: What Survived and How

### 1. Executable Living Specifications (CORE DIFFERENTIATOR)

**Original proposal:** Specs as executable programs using an embedded DSL (Rhai/Starlark) that verify their own fulfillment.

**What survived:** The executable concept — but with a radically simpler implementation. No custom language. Instead, specs use a dual-track criteria model:

- **Deterministic criteria:** Shell commands, test references, linter checks — binary, reproducible, cheap
- **Agent-evaluated criteria:** Natural-language assertions evaluated by an AI agent — nuanced, context-aware, handles what tests can't express

**Key insight from debate:** Agents ARE the execution engine. Natural-language acceptance criteria evaluated by agents are simpler and more powerful than any spec DSL. The challenger's insight eliminated months of language design work.

**Explorer's counter-insight:** Deterministic criteria aren't optional. Agent evaluation is non-deterministic and costly. Shell commands provide the reliable foundation. Both tracks are first-class.

**Proposed spec format:**

```toml
[spec.auth-001]
description = "User authentication via OAuth2"

[[spec.auth-001.criteria]]
type = "deterministic"
command = "cargo test --test auth_integration"

[[spec.auth-001.criteria]]
type = "agent-evaluated"
assertion = "All authentication error responses include actionable user guidance, not technical error codes"
confidence = "high"  # requires >= 2/3 consistent verdicts across runs
```

**Timeline:** 2-4 weeks for MVP (deterministic only); +2-4 weeks for agent-evaluated track.

**Why this is radical:** It kills spec drift by making specs self-verifying. It eliminates the "write spec, implement, write tests, discover spec was ambiguous" cycle. Agents can interrogate specs programmatically to understand requirements without human disambiguation.

---

### 2. Intent Provenance Chain (HIGH VALUE, LOW COST)

**Original proposal:** Sidecar database tracking an intent graph linking every code artifact back to its originating spec.

**What survived:** Convention-based provenance on top of git. No sidecar DB. No intent graph. No agent self-reporting.

**Key refinement from debate:**
- Explorer: Granularity should be at PR/branch level, not individual commits (agents generate noisy commit histories)
- Challenger: Spec-reference enforcement should be a built-in default gate, opt-out rather than opt-in

**Convention:**
- Branch names include spec ID: `spec/auth-001/implement-oauth`
- PR descriptions reference spec IDs (enforced by a default gate)
- `assay trace spec:auth-001` queries branches and PRs to show all linked code
- `assay drift` compares spec criteria against recent changes to linked files

**Timeline:** 1-2 weeks for core trace/drift commands.

**Why this survived:** It captures 80% of the value of the original proposal at 10% of the cost. Git already stores the history — Assay just adds conventions and queries.

---

### 3. Two-Pass Gated Review (REFINED FROM TRIBUNAL)

**Original proposal:** Three-agent adversarial tribunal (Advocate, Prosecutor, Judge).

**What survived:** Two-pass review for high-risk changes only.

**Key debate points:**
- Challenger correctly identified the "theater problem" — adversarial agents generate artificial disagreements
- Explorer's counter: single-pass review has a confirmation bias problem (anchoring on first impression)
- Compromise: Pass 1 is standard structured review; Pass 2 (challenge-the-approval) triggers only on approvals of high-risk changes

**Risk classification (rule-based, not agent-based):**
- Touches security-sensitive paths → high risk
- Changes public API signatures → high risk
- Exceeds N lines changed → high risk
- Everything else → low risk

**Important constraint:** This is Phase 2, not MVP. Single-pass structured review must work first. Two-pass is a configuration option layered on top.

**Structured review output format:**
1. Spec conformance assessment
2. Risks and concerns
3. Strengths
4. Verdict with rationale and remediations

---

### 4. Spec Export + Violation Feedback Loop (SCOPED DOWN FROM RUNTIME)

**Original proposal:** Specs become live production constraints, executing as runtime assertions alongside the application.

**What survived:** Two-part approach:
- **Spec export:** Generate monitoring artifacts FROM specs (Prometheus alert rules, SLO definitions, OPA policies)
- **Violation webhook:** Accept inbound signals when monitoring detects violations, linking them back to specs and optionally triggering agent investigation

**Key debate resolution:**
- Challenger correctly identified scope explosion risk — competing with Datadog/Prometheus is a losing fight
- Explorer's counter: the feedback loop (violation → agent investigation → fix proposal) is the novel part
- Compromise: export artifacts (monitoring tools do monitoring), but accept inbound violation signals to close the loop

**Sequencing:** Phase 3-4 feature. Depends on working specs, gates, agent integration, and export mechanism. Architecture should not prevent it (message-passing boundaries, serializable types).

---

### 5. Protocol-Ready Internal Architecture (DESIGN PRINCIPLE)

**Original proposal:** Agentic Development Protocol (ADP) — a formal open standard for agent-spec-gate-review communication.

**What survived:** A design principle, not a deliverable. Internal interfaces should be designed as if they'll become a protocol:
- Message-based boundaries between components
- Serializable types at every interface (already started with `assay-types` + serde + schemars)
- No hidden coupling between core logic and transport

**Key insight from debate:** Every successful protocol (LSP, MCP, HTTP) was extracted from working software, never designed in advance. Build the tool first, extract the standard second. If Assay gets adoption, protocol extraction is a refactoring exercise, not a rewrite.

---

### 6. Spec Versioning with Dependency Gates (SIMPLIFIED)

**Original proposal:** Full temporal branching system with parallel agent timelines and speculative execution.

**What survived:** Two fields and one gate type:
- `version: u32` on the Spec type
- `supersedes: Option<SpecId>` linking to the previous version
- `spec-dependency` gate type: spec v2 gates cannot pass until spec v1 gates have passed

```toml
[[gate]]
type = "spec-dependency"
requires = "auth-001@v1"
```

**Why the rest was killed:** Temporal branching is overengineered. Git already handles version control. The sequential gate constraint is the actually useful enforcement mechanism.

---

## What We Killed and Why

| Killed Proposal | Original Form | Reason for Killing |
|---|---|---|
| Agentic Development Protocol | Formal open standard with spec work | Can't standardize what you haven't built; premature standardization freezes bad abstractions |
| Custom Spec DSL | Rhai/Starlark embedded language | Agents + shell commands replace the need entirely; language design is a multi-year commitment |
| Three-Agent Tribunal | Advocate/Prosecutor/Judge roles | Theater problem: adversarial agents generate artificial disagreements; 3x cost for marginal insight |
| Sidecar Intent Database | Parallel data store with intent graph | Goes stale; depends on unreliable agent self-reporting; git conventions achieve 80% at 10% cost |
| Spec-as-Runtime Execution | Specs as live production assertions | Scope explosion into observability; competing with Datadog/Prometheus is a losing fight |
| Agent Capability Marketplace | Intelligent task routing between agents | YAGNI: zero agent integrations working; plugin system already provides capability registration |
| Temporal Branching System | Custom temporal branches with parallel timelines | Overengineered; version field + supersedes + dependency gate captures the essence |

---

## Implementation Sequencing

### Phase 1 — Foundation (MVP that proves the concept)
- Spec format (TOML) with description + deterministic criteria
- Gate engine that evaluates deterministic criteria (shell commands, test references)
- `assay check` CLI command — evaluate all gates for a spec
- Spec-linked branch/PR convention with enforcement gate
- `assay trace` — query which code relates to which spec

### Phase 2 — Agent Integration (the differentiator)
- Agent-evaluated criteria (natural language assertions evaluated by agents)
- Structured single-pass review (spec conformance, risks, strengths, verdict)
- Plugin system for agent integration (Claude Code first, others follow)
- Confidence thresholds for non-deterministic evaluations

### Phase 3 — Maturity
- Two-pass gated review (challenge-the-approval on high-risk changes)
- Spec export to monitoring tools (Prometheus, SLO, OPA)
- Spec versioning with dependency gates
- Drift detection (`assay drift`)

### Phase 4 — Ecosystem
- Violation webhook / feedback loop
- Protocol extraction if adoption warrants it
- Additional agent plugins
- TUI for spec/gate/review visualization

---

## Key Takeaways

1. **The biggest insight wasn't in the original proposals.** "Agents as the spec evaluation engine" emerged from the challenger's critique and fundamentally reshaped the executable specs proposal. Adversarial debate works.

2. **Dual-track criteria is the unique positioning.** No existing tool combines deterministic (shell/test) and agent-evaluated (natural language) spec criteria. This is Assay's category-defining feature.

3. **Convention over infrastructure.** Git-based provenance, rule-based risk classification, TOML-based specs — the best solutions leveraged existing tools and conventions rather than building new infrastructure.

4. **Scope discipline matters more than vision.** Five of seven proposals were correctly scoped down. The ambitious versions were interesting but would have spread a zero-code project across too many fronts. Build the core, prove it works, then expand.

5. **Design for extraction, not standardization.** Protocol-ready internal interfaces are a design principle. Protocol standardization is an outcome of success, not a path to success.
