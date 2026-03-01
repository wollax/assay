# AI-Powered Compression Ideas

**Explorer:** explorer-ai-compression
**Date:** 2026-02-28
**Focus:** Session memory compression and Haiku subagent tool output compression

---

## Idea 1: Gate Evidence Summarizer (Haiku Middleware)

**Name:** Gate Evidence Summarizer

**What:** An MCP middleware layer that intercepts `gate_run` responses before they reach the calling agent. Raw `Vec<GateResult>` payloads — which contain full stdout/stderr from test suites, linters, type checkers — are piped through a Haiku-class model that extracts only actionable information. Instead of 500 lines of `cargo test` output, the agent receives: "3/47 tests failed: `test_auth_flow` (assertion error: expected 200, got 401 at auth.rs:45), `test_token_expiry` (timeout after 5s), `test_login` (missing mock for UserService)".

The middleware sits in `assay-mcp` as a configurable response processor. It preserves the structured `GateResult` envelope (passed, exit_code, duration, timestamp) and only compresses the stdout/stderr evidence fields. The original evidence is written to a local evidence store (file or SQLite) keyed by a hash, so the full output is always recoverable.

**Why:**
- Gate evidence is the single largest token consumer in Assay's MCP pipeline. A `cargo test` run can produce 10-50KB of output per criterion. With 5-10 criteria per spec, a single `gate_run` response can be 100-500KB — most of which is noise (passing test names, timing info, compiler progress).
- Agents don't need raw evidence. They need: what failed, where, why, and what to do about it. Haiku can extract this at ~$0.001 per compression.
- This directly serves Assay's quality-gate mission: better compressed evidence means agents can iterate faster through the gate loop without exhausting context.
- Composable with the CLI surface: `assay gate run` (human) gets full output; MCP `gate_run` (agent) gets compressed output. Different consumers, different needs.

**Scope:** Medium (2-3 days)
- Haiku API integration in assay-mcp (~1 day)
- Evidence store for raw output preservation (~0.5 day)
- Compression prompt engineering and testing (~1 day)
- Configuration (enable/disable, threshold for when to compress) (~0.5 day)

**Risks:**
- **Lossy compression hides root causes.** The Haiku summary might omit a subtle detail from stderr that the agent needs to diagnose the actual problem (e.g., a deprecation warning that causes a downstream failure). Mitigation: always preserve raw evidence with a drill-down mechanism.
- **Latency penalty.** Every gate_run response pays a Haiku API round-trip (~200-500ms). For fast-failing gates (exit code 1 with 3 lines of output), this is overhead for no gain. Mitigation: byte-size threshold — only compress evidence above N bytes.
- **API dependency.** The MCP server now requires network access to an LLM API, which contradicts the "local-first" v0.1 design. Mitigation: make compression optional, default to off in v0.1, configurable per-project.
- **Prompt fragility.** Different gate types (test runners, linters, type checkers) produce radically different output formats. A single compression prompt may not handle all of them well. Mitigation: gate-kind-aware prompt templates.

---

## Idea 2: Session Memory Rings

**Name:** Session Memory Rings

**What:** A three-ring memory architecture for agent sessions managed by Assay's orchestrator:

- **Hot ring (working memory):** Current task context — the active spec, most recent gate results, current file edits. Kept in full fidelity. Size-bounded by a configurable token budget (e.g., 20K tokens).
- **Warm ring (session memory):** Decisions made, gates passed/failed, patterns noticed during this session. AI-compressed from hot ring when items age out. Stored as structured observations (key-value pairs with timestamps). ~5x compression ratio.
- **Cold ring (project memory):** Cross-session patterns, recurring failures, known solutions, architectural decisions. AI-compressed from warm ring at session end. Persisted to SQLite + optional vector DB. Injected into new sessions via retrieval. ~10x compression ratio.

The orchestrator manages ring transitions. When the hot ring exceeds its budget, the oldest items are compressed by Haiku into warm-ring observations. When a session ends, warm-ring observations are further compressed into cold-ring entries. New sessions receive relevant cold-ring entries selected by semantic similarity to the current spec.

**Why:**
- Agent sessions in Assay are long-running (a spec implementation might take 30+ tool calls). Context windows fill up. Without memory management, agents lose early context — including which gates they already tried, what failed, and why they changed approach.
- The ring model matches how human developers work: you remember what you're doing now (hot), what you decided earlier today (warm), and what you learned on past projects (cold).
- Assay's spec-driven workflow provides natural compression boundaries: each gate evaluation cycle is a discrete unit that can be summarized as an observation.
- Enables cross-session learning: if an agent failed a lint gate on Monday because of unused imports, and another agent hits the same gate on Wednesday, the cold ring provides "lint gate commonly fails on unused imports — run `cargo fix` before evaluating."

**Scope:** Large (1-2 weeks)
- Ring data model and storage (~2 days)
- Hot → warm compression logic with Haiku (~2 days)
- Warm → cold compression at session end (~1 day)
- Cold ring retrieval and injection (~2 days)
- Integration with Assay's session/orchestrator model (~2 days)

**Risks:**
- **Premature for v0.1.** Assay doesn't have a session/orchestrator model yet (it's a future milestone). Building memory rings before the thing that generates memories is premature architecture.
- **Compression cascade errors.** Bad hot→warm compression produces bad warm→cold entries, which inject bad context into future sessions. Error compounds across rings.
- **Vector DB complexity.** Cold ring retrieval via semantic search adds a heavyweight dependency (embedding model + vector store). SQLite full-text search may be sufficient for v0.1 and avoids the complexity.
- **Privacy/security.** Gate evidence may contain secrets (API keys in error output, credentials in test fixtures). Compressing and storing this data across sessions requires careful handling.

---

## Idea 3: Progressive Gate Disclosure

**Name:** Progressive Gate Disclosure

**What:** Redesign the MCP tool surface from a single `gate_run` tool into a two-tool disclosure pattern:

1. **`gate_run`** — Returns a compressed summary by default: per-criterion pass/fail, one-line reason for failures, aggregate stats ("4/6 criteria passed"). No raw stdout/stderr. This is what the agent sees on first call. Total response: ~500 tokens regardless of how many criteria or how verbose their output.

2. **`gate_evidence`** — Drill-down tool that returns the full raw stdout/stderr for a specific criterion. The agent only calls this when the summary isn't enough to diagnose a failure. Input: `{ spec_name, criterion_name }`. Returns the original uncompressed evidence.

The compression happens synchronously in the `gate_run` handler: run all criteria, capture all output, compress via Haiku (or even deterministic extraction for structured output formats), return summary. Raw evidence is held in-memory or spilled to a temp file for the duration of the session.

**Why:**
- Follows the established pattern of progressive disclosure in information architecture. Most of the time, the agent only needs to know WHAT failed. Only sometimes does it need to see WHY.
- Token-efficient by default. An agent doing 5 gate evaluation cycles only pays full evidence tokens for the specific failures it investigates, not for all criteria on every cycle.
- No lossy compression anxiety — the full evidence is always one tool call away. The agent decides when fidelity matters, not the compression algorithm.
- Composes naturally with Assay's quality-gate mission: the summary IS the gate result (passed/failed with reason). Evidence is supporting documentation, not the verdict.
- Works without AI compression too: for well-structured output (JUnit XML, TAP format, JSON reporters), deterministic parsers can extract the summary without any LLM call.

**Scope:** Small-Medium (1-2 days)
- Split gate_run into summary + evidence tools (~0.5 day)
- Deterministic extractors for common output formats (~1 day)
- Optional Haiku fallback for unstructured output (~0.5 day)
- Evidence lifecycle management (cleanup after session) (~0.5 day)

**Risks:**
- **Agent might not know to drill down.** If the compressed summary is too terse, the agent might try to fix a failure based on incomplete information, waste a cycle, and then drill down. Mitigation: rich one-line reasons that include file:line when available.
- **Two tools instead of one increases tool surface.** MCP best practices suggest minimizing tool count. Counter-argument: two focused tools are better than one tool that returns variable-size responses.
- **Evidence lifecycle complexity.** How long is raw evidence kept? Per-session? Per-gate-run? If the agent calls `gate_evidence` after evidence has been cleaned up, it fails. Mitigation: re-run the single criterion on demand if evidence has expired.

---

## Idea 4: Cross-Session Gate Pattern Mining

**Name:** Gate Pattern Miner

**What:** An asynchronous background process that runs after each agent session completes. It analyzes the session's gate evaluation history — what criteria failed, what code changes the agent made to fix them, what eventually passed — and extracts reusable "gate patterns." These patterns are stored as structured entries:

```
Pattern: cargo-clippy-unused-import
Trigger: Clippy criterion fails with "unused import" warnings
Resolution: Agent ran `cargo fix --allow-dirty` then re-evaluated
Success rate: 4/4 sessions
Last seen: 2026-02-28
```

When a new session encounters a gate failure, the system searches for matching patterns and injects them as context: "This clippy failure has been resolved 4 times before by running `cargo fix --allow-dirty`."

Pattern mining uses a Haiku-class model to analyze the session trace (a log of MCP tool calls and their results), extract the failure→fix→success sequences, and generalize them into reusable patterns.

**Why:**
- Agent sessions in Assay are largely independent (different worktrees), but they encounter the same gate failures repeatedly. Without cross-session learning, each agent rediscovers the same solutions.
- Pattern mining is particularly valuable for Assay because gate criteria are STATIC (defined in the spec) while agent implementations VARY. This means the same criterion (e.g., `cargo test`) fails in predictable, categorizable ways across sessions.
- The mining process runs asynchronously after the session, so it adds zero latency to the agent's workflow.
- Patterns can be reviewed and curated by humans via the TUI dashboard, acting as a knowledge base for common issues.

**Scope:** Medium-Large (1-1.5 weeks)
- Session trace capture and storage (~2 days)
- Pattern extraction logic with Haiku (~2 days)
- Pattern matching and injection (~1-2 days)
- Pattern storage (SQLite) and CRUD via CLI/TUI (~1-2 days)

**Risks:**
- **Overfitting to environment.** Patterns extracted from one developer's machine (specific Rust version, OS, dependencies) may not generalize. A pattern that works on macOS fails on Linux CI.
- **Stale patterns.** A pattern that was correct last month may be wrong after a dependency update. Patterns need expiration/validation.
- **Misleading confidence.** "4/4 success rate" on a pattern doesn't mean it's correct for the current failure — correlation isn't causation. The agent might blindly apply a pattern when the root cause is different.
- **Session traces can be enormous.** A 30-tool-call session produces a large trace. Haiku compression of the full trace might cost more than the pattern is worth. Mitigation: only mine sessions that went through gate failure → fix → success cycles.

---

## Idea 5: Context Budget Allocator

**Name:** Context Budget Allocator

**What:** An orchestration-layer system that manages the "context budget" across N concurrent agent sessions. Each session gets a configurable token budget (e.g., 80K tokens for Sonnet context). The allocator monitors each session's context usage and triggers progressive compression when sessions approach their limits.

Compression priority order (least-valuable-first):
1. **Old passing gate results.** "All 47 tests passed" from 5 cycles ago → compressed to "Tests: passed (cycle 3)"
2. **Superseded decisions.** "Tried approach A, abandoned it" → compressed to "Approach A rejected: didn't handle edge case X"
3. **Completed spec sections.** Once a criterion passes and isn't revisited, its evidence is compressed
4. **Stable context.** Spec content, config, architecture notes → moved to system prompt / persistent context

The allocator is spec-aware: evidence for FAILING gates is NEVER compressed (the agent needs it). Evidence for PASSING gates is aggressively compressed (the agent just needs confirmation). This asymmetry is key — Assay knows which gates are failing because it runs them.

The allocator communicates with agents via the MCP server, providing a `context_status` resource that agents can check, and a `compact_context` tool that agents can call to proactively free up space.

**Why:**
- Context exhaustion is the #1 failure mode for long-running agent sessions. When context fills up, the platform's auto-compressor kicks in and compresses everything indiscriminately — losing gate evidence, spec context, and decisions that the agent still needs.
- Assay has unique knowledge about what's important (failing gate evidence) and what's not (passing gate boilerplate). No generic compressor has this domain knowledge.
- The allocator works WITH platform compaction, not against it: by proactively compressing low-value content, it delays platform compaction and preserves high-value content longer.
- Scales with the orchestration vision: when Assay manages 10 concurrent sessions, each with different specs and different failure patterns, per-session budget management becomes critical.

**Scope:** Large (2+ weeks)
- Context usage monitoring (~2 days)
- Compression priority engine (~3 days)
- Spec-aware value scoring (~2 days)
- Integration with MCP resource/tool surface (~2 days)
- Integration with orchestrator session management (~3 days)
- Testing across different session patterns (~2 days)

**Risks:**
- **Tight coupling to platform internals.** Monitoring "context usage" requires knowing how the host platform (Claude Code, Codex) manages context — which is platform-specific and undocumented. Different platforms compact differently.
- **Over-engineering for v0.1.** This is an orchestration-level feature, but v0.1 doesn't have an orchestrator. It's a v0.3+ feature at the earliest.
- **Second-system syndrome.** Building a "smart" allocator is tempting but the simpler approach (just compress old gate results deterministically) might capture 80% of the value at 20% of the complexity.
- **Context accounting is approximate.** Token counts vary by model, tokenizer, and content type. The budget can only be approximate, making the "approaching limit" trigger unreliable.

---

## Idea 6: Haiku-Powered Tool Response Middleware

**Name:** MCP Compression Middleware

**What:** A generic middleware layer in `assay-mcp` that sits between ALL MCP tool handlers and the calling agent. Any tool response above a configurable byte threshold is automatically routed through a Haiku-class model for compression before being sent to the calling agent. The middleware is tool-agnostic — it works with `spec_get`, `gate_run`, `spec_list`, and any future tools.

The middleware consists of three components:
1. **Size gate:** Responses below threshold (e.g., 2KB) pass through uncompressed.
2. **Compressor:** Haiku model with a tool-aware prompt that knows the response schema and extracts the agent-relevant subset. The prompt includes the tool name and input parameters as context, so the compressor knows WHAT the agent asked for.
3. **Provenance header:** Compressed responses include a `_compressed: true` field and a `_evidence_id` reference to the full response stored locally. The agent can call a generic `get_evidence` tool with the ID to retrieve the original.

Configuration is per-tool:
```toml
[compression]
enabled = true
threshold_bytes = 2048

[compression.gate_run]
enabled = true
threshold_bytes = 1024  # Gates are high-value, compress earlier
prompt_template = "gate_evidence"

[compression.spec_get]
enabled = false  # Specs are small, don't compress
```

**Why:**
- Generic middleware is more maintainable than per-tool compression logic. As Assay adds tools (Phase 8 has `spec_get`, `gate_run`, `spec_list`; future phases add more), compression scales automatically.
- Per-tool configuration lets Assay optimize compression for each tool's characteristics. Gate evidence needs aggressive compression with actionable summaries. Spec content should never be compressed (it's the agent's task definition). Future tools get sensible defaults.
- The provenance header pattern is borrowed from HTTP (Content-Encoding) — the agent knows the response was compressed and can choose to drill down.
- Centralizes the Haiku API integration in one place. Tool handlers don't need to know about compression; they return full-fidelity responses and the middleware handles the rest.

**Scope:** Medium (3-5 days)
- Middleware trait/function in assay-mcp (~1 day)
- Haiku API client with retry/fallback (~1 day)
- Per-tool compression configuration (~0.5 day)
- Evidence store and `get_evidence` tool (~1 day)
- Prompt templates for different tool types (~0.5-1 day)

**Risks:**
- **Generic compression may be worse than specialized.** A gate-evidence-specific compressor knows about test output formats, assertion patterns, file:line references. A generic compressor loses that domain knowledge. Mitigation: tool-specific prompt templates, but this erodes the "generic" benefit.
- **All-or-nothing latency.** Every above-threshold response pays Haiku latency. The agent can't opt out for a specific call ("give me the full gate_run this time"). Mitigation: add a `compress: false` parameter on tool inputs that bypasses the middleware.
- **Error amplification.** If Haiku is down or returns garbage, the middleware must fall back gracefully to the uncompressed response. The "smart" path must never be worse than the "dumb" path.
- **Testing complexity.** Middleware between tool handlers and MCP transport is an integration testing pain point — you need to test compressed and uncompressed paths for every tool.

---

## Summary Table

| # | Name | Type | Scope | Key Insight |
|---|------|------|-------|-------------|
| 1 | Gate Evidence Summarizer | Haiku middleware | Medium | Compress the biggest token consumer: raw gate stdout/stderr |
| 2 | Session Memory Rings | Session memory | Large | Three-tier memory with AI-powered promotion/demotion |
| 3 | Progressive Gate Disclosure | Tool design | Small-Medium | Two-tool pattern: summary by default, evidence on demand |
| 4 | Gate Pattern Miner | Cross-session learning | Medium-Large | Extract reusable failure→fix patterns across sessions |
| 5 | Context Budget Allocator | Orchestration | Large | Spec-aware context management that preserves failing-gate evidence |
| 6 | MCP Compression Middleware | Generic middleware | Medium | Tool-agnostic compression layer with per-tool configuration |

---

*Explorer: explorer-ai-compression | Date: 2026-02-28*
