# AI-Powered Compression — Final Report

**Explorer:** explorer-ai-compression
**Challenger:** challenger-ai-compression
**Date:** 2026-02-28
**Rounds:** 3 (proposals → critique → defense/synthesis → convergence)

---

## Executive Summary

Six AI-powered compression proposals were explored, challenged, and pressure-tested across three debate rounds. The core question: how should Assay manage token-expensive content (primarily gate evidence) in its MCP tool responses?

**Answer: Good API design, not AI compression.** Assay should produce focused, bounded-size MCP responses using deterministic summary extraction with drill-down capability. AI-powered compression is deferred until Assay already has an LLM API dependency for other reasons (agent-evaluated criteria in v0.2+). Context window management is the platform's job — Assay's job is producing responses that are inherently token-efficient.

One proposal adopted, one absorbed, two deferred, two killed.

---

## Adopted: Progressive Gate Disclosure (Phase 8)

### Design

Split gate evaluation into a two-tool MCP pattern:

**`gate_run`** — Returns a bounded-size structured summary:
```json
{
  "spec_name": "add-auth-flow",
  "summary": "4/6 criteria passed",
  "run_id": "run-001",
  "criteria": [
    { "name": "tests-pass", "passed": true, "exit_code": 0, "reason": null },
    { "name": "clippy-clean", "passed": false, "exit_code": 1, "reason": "3 warnings: unused import (auth.rs:7), dead code (lib.rs:42), needless borrow (main.rs:15)" },
    { "name": "fmt-check", "passed": true, "exit_code": 0, "reason": null },
    { "name": "auth-endpoint", "passed": false, "exit_code": 1, "reason": "assertion failed: expected 200, got 404 at tests/auth_test.rs:28" }
  ],
  "duration_ms": 12340,
  "timestamp": "2026-02-28T23:30:00Z"
}
```

**`gate_evidence`** — Returns full raw stdout+stderr for a specific criterion:
```json
{
  "input": { "spec_name": "add-auth-flow", "criterion_name": "clippy-clean", "run_id": "run-001" },
  "output": { "stdout": "...(full clippy output)...", "stderr": "...(full stderr)...", "exit_code": 1 }
}
```

### Reason Field Extraction Strategy

Deterministic, tool-ecosystem-aware parsing (no AI required):

1. **Exit code 0 (passed):** `reason = null` — no reason needed for passing criteria
2. **Exit code non-zero (failed):** `reason = last_meaningful_lines(stderr, stdout)` where "meaningful" means:
   - Skip blank lines, progress indicators ("Compiling...", "Running..."), and timing info
   - Take the last N non-noise lines from whichever stream contains the failure information
   - `cargo test` puts summaries at the end of **stdout** (not stderr)
   - `cargo clippy` puts warnings on **stderr** but summary count on **stdout**
   - The extractor must be stream-aware, not just "take stderr"
3. **Phase 8+ enhancement:** Known format parsers for JUnit XML, TAP, JSON reporters, cargo test native output — extract structured failure information instead of raw line slicing

### Evidence Lifecycle

- Evidence stored **in-memory** within the MCP server process
- **All runs retained** (not just latest) — enables cross-run comparison ("this test was failing last run, what was the error?")
- Memory bounded: ~10 runs × 8 criteria × 50KB = 4MB max per session — negligible
- Evidence dies with the server process (stdio transport = one client per process)
- No SQLite, no file persistence, no cleanup logic
- `run_id` parameter on `gate_evidence` allows drilling into any retained run

### Why This Works

- **Interface Segregation Principle applied to MCP tools.** Returning 50KB of raw stdout from 8 criteria in a single response is bad API design regardless of context management. Assay knows the structure (per-criterion pass/fail, exit codes, stderr vs stdout) and can produce a summary that's not just smaller but more informative per token.
- **No AI dependency.** No Haiku API calls, no API keys, no reqwest, no retry logic. Pure deterministic parsing in Rust.
- **No lossy compression anxiety.** Full evidence is always one tool call away. The agent decides when to drill down, not the compression algorithm.
- **80/20 capture.** Most gate criteria produce structured output (test runners, linters, type checkers). Deterministic parsers handle these reliably and at zero cost.

### Scope

Small-medium. This is a Phase 8 (MCP Server Tools) design decision, not a standalone feature. When implementing `gate_run`, build it as the two-tool pair from the start.

---

## Absorbed: Gate Evidence Summarizer → Progressive Gate Disclosure

**Original proposal:** Haiku middleware intercepting `gate_run` responses for AI summarization.

**Why killed as standalone:** Hallucination in the critical path is a fundamental reliability issue for a quality-enforcement tool. A Haiku summary will capture obvious assertion failures but miss subtle contextual clues (test ordering dependencies, deprecation warnings causing downstream failures, environment-specific notes). In a tool whose entire purpose is quality enforcement, the evidence IS the product.

**What survived:** The "cross-run comparison" use case (what changed between gate runs?) is genuinely useful, but it's a deterministic diff of two `GateResult` structs — no AI needed. "test_foo: FAIL→PASS, test_bar: PASS→FAIL" is trivial structural comparison. This capability is naturally supported by the progressive disclosure pattern's multi-run evidence retention.

---

## Deferred: AI-Powered Evidence Summarization (v0.2)

**Trigger:** When agent-evaluated criteria (the `prompt` field on Criterion) are implemented in v0.2, Assay will already have an LLM API integration (reqwest, API types, retry logic, key management). At that point, the marginal cost of adding AI-powered evidence summarization is near-zero.

**Implementation direction:** Opt-in `ai_summary` field on the `gate_run` response, alongside the deterministic `reason`. The agent gets both: deterministic summary always, AI-enhanced summary when available and enabled. NOT a generic middleware wrapper — implement as a specific enhancement to the gate_run tool to avoid the testing matrix explosion of a middleware approach.

**Why not now:** Building a mini AI SDK in Rust (reqwest + custom Anthropic API types + retry + circuit breaker + async HTTP + API key management) for a v0.1 project with empty module stubs is unjustified. The dependency arrives naturally with agent-evaluated criteria.

---

## Deferred: Gate Pattern Mining (v0.3+)

**Trigger:** When the orchestrator and session model exist, enabling session traces to be captured and analyzed.

**Concept:** Analyze completed session traces to extract reusable failure→fix→success patterns. Inject matching patterns as context when new sessions encounter similar gate failures.

**Why not now:**
- No sessions exist (v0.1 has no session model or orchestrator)
- No traces to mine
- Agents already have generic learning (Claude Code `/memory`)
- Domain-specific learning (patterns tied to specific gate criteria) will likely outperform generic learning, but this needs the orchestrator to prove

**Open question for v0.3:** Does domain-specific gate pattern mining outperform the platform's generic learning enough to justify the investment? This requires comparative data from the orchestrator.

---

## Killed: Session Memory Rings

**Proposal:** Three-tier memory architecture (hot/warm/cold) with AI-powered promotion/demotion between rings.

**Why killed:**
1. **Premature by 2+ milestones.** No session model, no orchestrator, no runtime to generate memories.
2. **Compression cascade.** Hot→warm→cold compression compounds information loss across tiers. In a quality-enforcement tool, injecting degraded context into future sessions means injecting wrong context.
3. **Architectural astronautics.** Building a custom RAG system (vector DB, embedding model, semantic retrieval) in a CLI tool that currently prints its version number and exits.
4. **Privacy/security.** Gate evidence can contain secrets (API keys in error output, credentials in test fixtures). Persisting across sessions without encryption is a security incident waiting to happen.

---

## Killed: Context Budget Allocator

**Proposal:** Orchestration-level system managing per-session token budgets with spec-aware compression priority.

**Why killed:**
1. **Wrong abstraction layer.** The agent's context window is managed by the PLATFORM (Claude Code, Codex). Assay's MCP server has no visibility into agent context usage. MCP is a one-way tool interface — the agent calls the server, not the other way around.
2. **Two competing compressors.** If Assay independently manages context while the platform's auto-compactor also manages context, they interfere destructively.
3. **Spec-aware heuristics are fragile.** "Failing evidence preserved, passing evidence compressed" sounds correct but breaks when the agent needs passing test output as a reference template for writing new tests.
4. **Approximate accounting.** Token counts are model-specific and tokenizer-dependent. Building a 2-week feature on approximate metrics is not justified.

**Preserved insight:** The spec-aware value heuristic (failing evidence is more important than passing evidence) should inform the progressive disclosure design: failing criteria get richer `reason` summaries, passing criteria get minimal representation (`reason: null`).

---

## Architectural Principle: Bounded MCP Responses

Established through debate as a concrete, testable design guideline for all Assay MCP tools:

> **MCP Response Design Guideline:** Every Assay MCP tool response must be bounded in size regardless of the underlying data volume. Tools that produce variable-size output (gate evidence, spec listings) MUST use a summary+drill-down pattern: the default response is a fixed-structure summary, and a companion tool provides full-fidelity data on demand.

This principle:
- Turns "good API design" into a reviewable constraint
- Applies to all current and future MCP tools (not just `gate_run`)
- Is testable: any new tool can be reviewed against it
- Encodes Assay's domain knowledge (it knows what's actionable) into API structure
- Establishes that context management is the platform's job, while response design is Assay's job

---

## Key Debate Insights

### 1. "Compression" vs "API Design"

The most important outcome of this brainstorm: what we initially framed as "AI-powered compression" is actually an API design problem. The token savings come not from AI summarization but from structuring responses so agents get what they need by default and can drill down when necessary. This is the Interface Segregation Principle, not machine learning.

### 2. Assay's Unique Advantage

Assay has domain knowledge that no generic compressor has: it knows which gates are failing, what the spec criteria are, what the agent is trying to achieve, and what constitutes "actionable" in gate output. This knowledge should be expressed through response structure, not through AI post-processing.

### 3. AI Integration Arrives Naturally

The LLM API dependency that all AI compression proposals require will arrive naturally with agent-evaluated criteria (v0.2, the `prompt` field on Criterion). At that point, AI-powered enhancements to the progressive disclosure pattern become low-marginal-cost additions rather than expensive new infrastructure.

### 4. Whose Problem Is Context Management?

Settled: Assay produces focused, well-structured responses. Context window management is the platform's responsibility. Assay should never try to manage the agent's context window or compete with platform compaction.

---

## Summary Table

| # | Proposal | Verdict | Timing | Key Insight |
|---|----------|---------|--------|-------------|
| 3 | Progressive Gate Disclosure | **ADOPT** | Phase 8 | Deterministic two-tool pattern; good API design IS compression |
| 1 | Gate Evidence Summarizer | **ABSORBED** | — | Hallucination in critical path kills it; useful parts folded into Idea 3 |
| 6 | MCP Compression Middleware | **DEFER** | v0.2 | Arrives with agent-eval criteria; implement as opt-in field, not middleware |
| 4 | Gate Pattern Miner | **DEFER** | v0.3+ | Needs orchestrator and sessions to exist |
| 2 | Session Memory Rings | **KILL** | — | Premature by 2+ milestones; privacy risk; architectural astronautics |
| 5 | Context Budget Allocator | **KILL** | — | Wrong abstraction layer; platform's job |

---

*Report finalized: 2026-02-28*
*Explorer: explorer-ai-compression | Challenger: challenger-ai-compression*
