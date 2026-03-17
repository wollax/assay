# Brainstorm Summary: Assay Post-v0.1.0

**Date:** 2026-03-02
**Pairs:** 3 (quick-wins, high-value, radical)
**Rounds:** 2-3 per pair
**Status:** Quick-wins written by team lead (agent stalled); high-value and radical completed via debate

---

## Key Findings

### The Vision Is Right — Execute It

The radical track independently validated that the existing vision (orchestrator, merge-back, dual-track gates, TUI) is the correct product. No pivot warranted. The brainstorm's value was in **scoping and sequencing** the next concrete steps, not redirecting.

### The Breakthrough: Agent Gate Inversion

The single most important insight from this session: **agent-evaluated gates should receive evaluations from running agents, not call LLMs directly.** This eliminates an entire subsystem (HTTP client, API keys, retry, rate limiting) while preserving the category-defining differentiator. The trust problem (self-evaluation bias) has a pragmatic v0.2 answer: structured rigor + human audit trail via history. Independent evaluator enforcement waits for the orchestrator (v0.3).

### Convergence Across All Three Tracks

All three tracks independently arrived at **Run History / Gate History** as a v0.2 priority. Quick-wins identified test coverage and MCP hardening. High-value identified the 3-feature core. Radical extracted design principles. They're complementary, not competing.

---

## Surviving Proposals by Category

### Quick Wins (low effort, high impact)

| # | Proposal | Scope | Risk |
|---|----------|-------|------|
| 1 | Issue burn-down: CLI polish (~15 issues) | Small | Low |
| 2 | MCP hardening (9 issues, esp. timeout param) | Small | Low |
| 3 | Type system hygiene (serde, OutputDetail, invariants) | Small-Med | Medium |
| 4 | Test coverage gaps (MCP has 0 handler tests) | Small-Med | Low |
| 5 | Tooling tightening (cargo-deny warn→deny) | Small | Very Low |
| 6 | Dogfooding checkpoint (use Assay to build Assay) | Small | Low |

[Full report](quickwins-report.md)

### High-Value Features (the v0.2 core)

| # | Proposal | Scope | Risk | Dependencies |
|---|----------|-------|------|--------------|
| 1 | **Run History** — JSON persistence in `.assay/results/` | 200-300 lines | Low | None |
| 2 | **Required/Advisory gates** — `enforcement` enum on Criterion | 100-150 lines | Low | Schema bump |
| 3 | **`gate_report` MCP tool** — agents submit evaluations | 300-400 lines | Medium | Run History |
| 4 | Wire `FileExists` (connect dead code) | 50-100 lines | Low | None |

[Full report](highvalue-report.md)

### Radical Directions

| # | Proposal | Outcome |
|---|----------|---------|
| 1 | Agent Accountability Protocol | **Killed** — premature standardization |
| 2 | Swarm Forge (competitive multi-agent) | **Deferred** — needs orchestrator first |
| 3 | Gates Everywhere (domain-agnostic) | **Design principle** — already true, don't regress |
| 4 | Agent Pipelines (CI/CD for agents) | **Killed** — scope explosion, can't compete with incumbents |
| 5 | Gate History (provenance/persistence) | **Adopted** — converges with Run History from high-value track |

[Full report](radical-report.md)

---

## Cross-Cutting Themes

1. **Persistence is infrastructure.** All tracks independently identified that gate results must be persisted before any advanced features make sense.

2. **Invert dependencies to reduce scope.** Don't build subsystems when you can receive data from existing actors (agents already have LLM access).

3. **Trust through transparency, not algorithms.** Self-evaluation + audit trail is pragmatic. Confidence scores and consensus voting are research problems, not engineering problems.

4. **Keep types domain-agnostic.** The architecture already supports non-code domains. Don't narrow it.

5. **Pipeline semantics for the orchestrator.** When building the orchestrator, model it as stages with transition conditions, not ad-hoc control flow.

---

## Recommended v0.2 Sequencing

### Foundation (do first)

1. Dogfooding checkpoint — surfaces real UX issues
2. Test coverage gaps — safety net for feature work
3. Type system hygiene — stable foundation

### Core Features (the v0.2 differentiators)

4. Run History (persistence)
5. Required/Advisory gates
6. `gate_report` MCP tool (agent-evaluated gates)

### Polish (interleave or follow)

7. MCP hardening
8. CLI issue burn-down
9. Tooling tightening
10. Wire FileExists

**Estimated total:** ~650-950 lines of new feature code + ~500-800 lines of fixes/tests

---

## Open Design Questions

1. **`optional: bool` vs `enforcement: "required" | "advisory"`** on Criterion — explorer argues enum (readability), challenger argues boolean (YAGNI). Team lead decides.

2. **Run History file layout** — one file per run vs one file per spec. High-value track recommends one-per-run (avoids concurrent write issues). Radical track notes the layout constrains Phase 2 query capabilities — design thoughtfully.

3. **`gate_report` schema** — flat `{ spec_name, criterion_name, passed, reasoning, evaluator_role }` vs richer structure. Start flat, extend later.

---

## Confirmed Kills (Do Not Revisit)

- Agent Accountability Protocol (premature standardization)
- Agent Pipelines / CI system (scope explosion, unwinnable competition)
- Trust calibration / consensus mode (research problem)
- Composite gate logic (required/advisory delivers the value)
- Built-in LLM client (may never be needed)
- Async pipeline bifurcation (no consumer)

## Deferred (Revisit When Prerequisites Met)

- Swarm Forge / competitive multi-agent → needs orchestrator + agent gates + cheaper tokens
- Context-controlled evaluation (`gate_evaluate`) → needs context assembly design
- Independent evaluator enforcement → needs orchestrator
- SpecProvider trait → needs a second concrete provider
- TUI dashboard → needs orchestrator

---

*Synthesized from 3 explorer/challenger pairs — 2026-03-02*
