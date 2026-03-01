# Brainstorm Summary

**Date:** 2026-02-28
**Session:** 3 explorer/challenger pairs, 3 rounds of debate each
**Subject:** Assay — agentic development kit

---

## Quick Wins (Low-Effort, High-Impact)

5 proposals, ~10 hours total. Ordered by dependency chain.

| # | Proposal | Effort | Key Decision |
|---|----------|--------|-------------|
| 1 | Error Type Foundation | 1.5hrs | Unified `AssayError` with `#[non_exhaustive]`, structured `Validation { field, message }` |
| 2 | Schema Generation Pipeline | 1hr | Standalone binary + `just schemas`, not a build script |
| 3 | Config Loading + CLI `init` | 2-3hrs | TOML only, template-based init (not serialized), `toml` dep in assay-core |
| 4 | Spec Validation + CLI `validate` | 2-3hrs | `Spec::new()` returns Result, private fields + getters, trim-then-validate |
| 5 | Gate Enum Dispatch | 2.5-3hrs | `GateKind` enum, `passed: bool` removed entirely, sync `evaluate_gate()` |

**Cut:** Stub CLI subcommands (merged into real commands), plugin skill (deferred — capstone, not quick win).

[Full report](quickwins-report.md)

---

## High-Value Features (Substantial Investment)

7 features sequenced into 6 milestones. MVP = milestones 1-4.

| Milestone | Feature | Scope |
|-----------|---------|-------|
| 1 | Config & Persistence (`.assay/` directory) | ~1 milestone |
| 2 | Spec Engine (Markdown+TOML frontmatter) | ~1-2 milestones |
| 3 | MCP Server (stdio, 2-4 tools) | ~1-2 milestones |
| 4 | Gate Framework (command+file+threshold) + Claude Code Plugin | ~1-2 milestones |
| 5 | Workflow State Machine (single hardcoded workflow) | ~1 milestone |
| 6 | Review System (structured criteria, single reviewer) | ~1 milestone |

**North star:** "Agent reads a spec, does work, hits a gate, gets a result."

**Key debate outcome:** MCP server moved from position 6 to position 3 — the agentic differentiator ships early.

**Cut:** Plugin SDK (build by hand first, extract later), spec dependency graphs, workflow templates, review rubrics/weights.

[Full report](highvalue-report.md)

---

## Radical / Paradigm Shifts

7 proposals explored, 2 paradigm-shifting insights survived.

| # | Proposal | Status | Key Insight |
|---|----------|--------|-------------|
| 1 | Dual-Track Executable Specs | **CORE DIFFERENTIATOR** | Deterministic criteria (shell commands) + agent-evaluated criteria (natural language). Nobody else has both. |
| 2 | Intent Provenance Chain | High value, low cost | Convention-based on git (branch names, PR refs), not a sidecar DB. `assay trace` + `assay drift`. |
| 3 | Two-Pass Gated Review | Phase 2 | Challenge-the-approval on high-risk changes only. Rule-based risk classification. |
| 4 | Spec Export + Violation Feedback | Phase 3-4 | Export monitoring artifacts FROM specs. Accept violation webhooks to close the loop. |
| 5 | Protocol-Ready Architecture | Design principle | Message-based boundaries, serializable types. Extract protocol later, don't design it now. |
| 6 | Spec Versioning | Simplified | `version` + `supersedes` fields + dependency gate. |

**Killed:** Custom spec DSL (agents replace it), three-agent tribunal (theater problem), sidecar intent DB (git conventions are enough), spec-as-runtime (scope explosion), agent marketplace (YAGNI), temporal branching (overengineered).

**Biggest insight:** "Agents ARE the spec evaluation engine" — emerged from challenger critique. Eliminates months of language design.

[Full report](radical-report.md)

---

## Cross-Cutting Themes

1. **Agents first, not last.** All three pairs independently converged on prioritizing agent integration early. The agentic capability IS the product differentiation.

2. **Convention over infrastructure.** Git-based provenance, TOML-based specs, enum dispatch over trait objects — leverage existing tools rather than building new infrastructure.

3. **Dual-track criteria is the category definer.** Deterministic + agent-evaluated spec criteria is a novel combination no competitor offers.

4. **Scope discipline.** Across all pairs, ~60% of original proposals were correctly scoped down or killed. The best ideas got sharper through debate.

5. **Design for extraction.** Build working software first. Extract protocols, SDKs, and abstractions from real usage patterns.

---

## Recommended Sequencing

**Phase 1 — Foundation (~10hrs):** Quick wins 1-5. Error types, schemas, config loading, spec validation, gate dispatch.

**Phase 2 — MVP Loop:** Config & persistence → Spec engine (Markdown+TOML frontmatter with dual-track criteria) → MCP server (stdio) → Gate framework + Claude Code plugin.

**Phase 3 — Process:** Workflow state machine → Structured review (single-pass).

**Phase 4 — Differentiation:** Agent-evaluated criteria, intent provenance, two-pass review, spec versioning.

**Phase 5 — Ecosystem:** Spec export, violation webhooks, additional plugins, TUI visualization.
