# Brainstorm Summary: Token Compression for Assay

**Date:** 2026-02-28
**Topic:** Token compression at two layers — orchestration (what managed agents see) and MCP (what calling agents receive)
**Pairs:** 3 explorer/challenger teams across deterministic compression, AI-powered compression, and architecture/radical directions
**Rounds:** 3 per pair

---

## Core Insight

**What we framed as "compression" is actually an API design problem.** The biggest token savings come not from post-hoc compression (filtering, AI summarization, middleware) but from designing outputs that are inherently compact. Assay has domain knowledge no generic compressor has — it knows pass/fail semantics, gate structure, and consumer intent. That knowledge should be expressed through response structure, not through compression layers.

**Corrected thesis:** "Compact output design should be a default, not a retrofit."

---

## Surviving Proposals

### Adopt in v0.1 (Design decisions baked into existing phases)

| Proposal | Source | Phase | Scope | Description |
|----------|--------|-------|-------|-------------|
| Streaming capture with byte budget | Deterministic | 7 | Small | Use `BufReader` + bounded capture instead of `Command::output()`. Exit-code-aware strategy: aggressive budget for passes, conservative with error-marker preservation for failures. |
| Truncation metadata on GateResult | Deterministic | 3 | Small | Add `truncated: bool` + `original_bytes: Option<u64>` to GateResult. Truncation is a fact about evidence, not a presentation concern. |
| Summary-first MCP response / Progressive Gate Disclosure | Deterministic + AI | 8 | Small-Med | Two-tool pattern: `gate_run` returns bounded summary (pass/fail per criterion, failure reasons), `gate_evidence` returns full stdout/stderr for drill-down. |
| Output Detail Levels | Architecture | 3 + 7 | Small | `OutputDetail` enum (Full/Standard/Compact) on gate config. Semantic verbosity control via match arm, not byte truncation. |
| Structured Wire Format | Architecture | 3 + 8 | Small | Separate wire (MCP, compact JSON) vs. display (CLI, human-readable) formats. Both derive from same `GateResult`. |
| Serde hygiene | Deterministic | 3 + 8 | Small | `#[serde(skip_serializing_if)]` on all Option/String/Vec fields. Mechanical, 10-30% savings. |

### Defer to v0.2 (Needs real data or natural dependency arrival)

| Proposal | Source | Trigger | Description |
|----------|--------|---------|-------------|
| Evidence Compressor | Deterministic | Measure v0.1 output sizes | ANSI stripping, blank line collapse, duplicate line counting. Build only if gates consistently produce >2K tokens of passing output. |
| Tool-aware parsers | Deterministic | Identify top-3 token-burning tools | Recognize cargo test, clippy, eslint output formats. Build only for tools with >70% compression potential. |
| AI evidence summarization | AI | Agent-eval criteria adds LLM dependency | Opt-in `ai_summary` field alongside deterministic `reason`. Arrives naturally with the `prompt` field on Criterion. |
| Diff-mode for iterative gate runs | Architecture | Observe real agent usage | Return only changes on repeated `gate_run` calls. Validated by real usage data, not hypothetical. |

### Defer to v0.3+ (Needs orchestrator)

| Proposal | Source | Trigger | Description |
|----------|--------|---------|-------------|
| Gate Pattern Mining | AI | Orchestrator + sessions exist | Analyze session traces for failure→fix→success patterns. Inject matching patterns as context. |

---

## Killed Proposals (12 total)

| Proposal | Source | Kill Reason |
|----------|--------|-------------|
| Consumer-Aware Output Profiles | Deterministic | Premature abstraction — trait hierarchy for zero consumers |
| Cross-Run Delta Mode | Deterministic | Agent context eviction makes delta refs unreliable; statefulness creates debugging nightmares |
| Gate Evidence Summarizer (Haiku middleware) | AI | Hallucination in quality-enforcement critical path; useful parts absorbed into Progressive Disclosure |
| Session Memory Rings | AI | Premature by 2+ milestones; privacy risk; architectural astronautics |
| Context Budget Allocator | AI | Wrong abstraction layer — context management is the platform's job |
| MCP Compression Middleware | AI | Middleware approach creates testing matrix explosion; implement as specific tool enhancement instead |
| Token Budget System | Architecture | Over-engineered; token counting without tokenizer is unreliable; escape hatches undermine the budget |
| Composable Middleware Pipeline | Architecture | Contradicts "enum dispatch, not trait objects" convention; premature abstraction |
| Bidirectional "Assay Lens" | Architecture | Solves already-handled problems; MCP has typed responses |
| Context-Pressure-Adaptive System | Architecture | Control systems engineering for a supporting concern; insight collapsed into Output Detail Levels |
| Information Fidelity Gate | Architecture | Circular — structural fidelity is a unit test, semantic fidelity doubles AI costs |

---

## Cross-Cutting Themes

1. **Compression is a supporting concern, not a product.** Six architectural proposals for compression in a project with 41 unfinished requirements is scope inversion. The surviving ideas are design decisions, not features.

2. **Assay and RTK don't overlap.** RTK compresses raw shell output at the hook level. Assay produces structured gate results at the application level. Different layers, complementary. Users can use both.

3. **AI compression arrives naturally.** The LLM API dependency all AI proposals require will arrive with agent-evaluated criteria (v0.2, `prompt` field on Criterion). At that point, AI enhancements are low-marginal-cost additions.

4. **Context management is the platform's job.** Assay produces focused, well-structured responses. The agent's platform (Claude Code, Codex) manages the context window. Assay should never try to manage agent context or compete with platform compaction.

5. **Bounded MCP responses as architectural principle.** Every Assay MCP tool response must be bounded in size. Tools producing variable-size output use summary+drill-down: default response is a fixed-structure summary, companion tool provides full-fidelity data on demand.

---

## Estimated Token Savings (v0.1)

| Technique | Savings | Confidence |
|-----------|---------|------------|
| Summary-first MCP responses | 40-70% | High |
| Streaming byte budget | 50-80% on verbose tools | High |
| Serde skip_serializing_if | 10-30% | High |
| Output Detail Levels (Standard) | 30-50% | Medium |

**Combined v0.1 potential:** 50-80% reduction on gate output reaching agents, with zero AI and zero external dependencies.

---

## Recommended Sequencing

All surviving v0.1 proposals are **design decisions for existing phases**, not new phases:

1. **Phase 3** (Error Types and Domain Model): Add `truncated`/`original_bytes` to GateResult, define `OutputDetail` enum, apply serde hygiene, design wire format types
2. **Phase 7** (Gate Evaluation): Implement streaming capture with byte budget, implement OutputDetail match arms
3. **Phase 8** (MCP Server Tools): Implement two-tool pattern (gate_run + gate_evidence), use wire format for responses

No additional phases needed. No scope expansion to v0.1.0 roadmap.

---

## Full Reports

- [Deterministic Compression](deterministic-report.md)
- [AI-Powered Compression](ai-compression-report.md)
- [Architecture & New Directions](architecture-report.md)

---

*Synthesized: 2026-03-01*
