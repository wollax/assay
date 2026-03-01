# Architecture & New Directions: Compression in Assay

**Explorer:** explorer-architecture
**Date:** 2026-02-28
**Focus:** Architectural integration of token compression — paradigm shifts, composable systems, and new directions beyond RTK/claude-mem

---

## Proposal 1: Token Budgets as First-Class Gate Configuration

### What

Introduce a `TokenBudget` concept into Assay's type system and gate configuration. Every gate, spec, and workflow gets an optional token budget that constrains how much context window is consumed when presenting results to agents. Instead of compressing after the fact, gates declare their budget upfront, and the output pipeline adapts to fit.

```toml
[gate]
name = "test-suite"
kind = "Command"
cmd = "cargo test"
token_budget = 500  # max tokens for this gate's result

[gate.budget_strategy]
priority = ["exit_code", "summary", "failures", "full_output"]
```

The gate evaluation pipeline progressively renders output until the budget is exhausted. Exit code and summary always fit. Failures get included next. Full stdout/stderr only if budget remains. This is **not** AI compression — it's structured progressive disclosure built into the data model.

`GateResult` gains a `budget_applied: Option<TokenBudgetReport>` field showing what was included vs. truncated, so agents can request the full output if they need it (via a follow-up MCP call like `gate/get_full_output`).

### Why

- Token budgets compose naturally with Assay's existing configuration model — they're just another field on gates
- Deterministic, zero-cost at runtime (no AI calls, no external dependencies)
- Aligns with Assay's "types are DTOs" philosophy — `TokenBudget` is a serializable config value
- Enables the orchestrator to plan context window allocation across N concurrent sessions: "agent A gets 4K for gate results, agent B gets 8K"
- Makes compression visible and auditable — agents see what was trimmed, not silently lost
- Forward-compatible with AI compression: a budget strategy could include `summarize_with_model` as a late-stage fallback

### Scope

Medium — touches assay-types (new config types), assay-core (budget-aware rendering), and MCP (budget-aware tool responses). No new crate needed.

### Risks

- Token counting without a tokenizer is approximate (char-based heuristic). Could misjudge by 20-30%. Mitigation: conservative estimates, document the approximation.
- Adds configuration surface area. Users who don't care about tokens pay cognitive cost. Mitigation: budgets are optional with sensible defaults.
- Budget priority ordering is opaque — what if the agent needs stderr but the budget cut it? Mitigation: `gate/get_full_output` escape hatch.

---

## Proposal 2: Compression as a Composable Middleware Pipeline

### What

Define compression as a trait-based middleware stack that processes `GateResult` (and any MCP tool output) through a chain of transformers before it reaches the consumer. Each transformer implements a `Compressor` trait:

```rust
pub trait Compressor: Send + Sync {
    fn compress(&self, input: CompressorInput, ctx: &CompressionContext) -> CompressorOutput;
    fn name(&self) -> &str;
    fn is_deterministic(&self) -> bool;
}
```

The pipeline is ordered: deterministic compressors run first (RTK-style filtering, regex stripping, structured extraction), then optional AI compressors run last. This mirrors Assay's dual-track gate philosophy: deterministic first, agent-evaluated second.

```toml
[[compression.pipeline]]
kind = "StripAnsi"

[[compression.pipeline]]
kind = "CollapseWhitespace"

[[compression.pipeline]]
kind = "ExtractStructured"
format = "test-results"  # parses JUnit XML, cargo test output, etc.

[[compression.pipeline]]
kind = "AiSummarize"
model = "haiku"
max_tokens = 200
when = "budget_exceeded"  # only if deterministic steps weren't enough
```

The `CompressionContext` carries the token budget (from Proposal 1), remaining context window, and metadata about the consuming agent. Each compressor can inspect upstream results and decide whether to act.

### Why

- Directly mirrors Assay's dual-track architecture — deterministic + agent-evaluated, same pattern at the compression layer
- RTK and claude-mem are both point solutions; this subsumes both as pipeline stages
- Composability means users configure compression per-project, per-gate, or per-agent — not a monolithic setting
- The `is_deterministic()` marker lets Assay enforce ordering invariants: deterministic compressors must come before AI compressors
- Pipeline is inspectable: every stage reports its token savings, creating an audit trail
- The trait boundary makes compression testable: mock compressors for unit tests, real compressors for integration

### Scope

Large — new trait in assay-core, pipeline executor, configuration model, several built-in compressors. Likely a new `compression` module in assay-core.

### Risks

- Over-abstraction this early. The v0.1 milestone doesn't even have gates running yet. Mitigation: define the trait and pipeline model now, ship only 1-2 deterministic compressors in v0.1 or v0.2.
- Ordering sensitivity: compressors that depend on raw output (e.g., structured extractors) must run before compressors that strip content. Mitigation: phase tags (`raw`, `structured`, `reduced`, `summarized`) to enforce ordering.
- AI compressors introduce latency and cost into the gate result pipeline. Mitigation: `when = "budget_exceeded"` conditional execution.

---

## Proposal 3: Bidirectional Compression — The "Assay Lens"

### What

Compression shouldn't just reduce what agents *receive* — it should also optimize what agents *send back* to Assay. When an agent reports gate results, submits a review, or proposes spec changes through MCP, the payload may be verbose (large code blocks, redundant explanations, conversational tone). The "Assay Lens" normalizes both directions:

**Inbound (Assay → Agent):** Standard compression — gate results, spec content, error messages are compressed before reaching the agent's context.

**Outbound (Agent → Assay):** Structured extraction — when an agent responds to an MCP tool call, Assay validates the response against the expected schema and extracts only the structured fields, discarding narrative wrapper text. For agent-evaluated gates, the response must conform to `AgentEvalResponse { passed: bool, evidence: String, confidence: f32 }` — any extra prose is stripped.

```
Agent sends:
"After careful analysis of the code, I believe the authentication
module meets the security criteria. The implementation uses bcrypt
with appropriate salt rounds. VERDICT: PASS with high confidence."

Assay Lens extracts:
{ passed: true, evidence: "bcrypt with appropriate salt rounds", confidence: 0.9 }
```

The lens is bidirectional but asymmetric: inbound uses the compression pipeline (Proposal 2), outbound uses schema-driven extraction. Both are configurable per-tool.

### Why

- Agent verbosity is a real cost. When the orchestrator manages N agents, the responses they send back consume tokens in the orchestrator's own context. Bidirectional compression reduces costs on both sides.
- Schema enforcement on agent responses improves data quality. Instead of parsing free-text for pass/fail, agents must conform to a structured contract.
- This builds directly on Assay's `AgentEvalRequest`/`AgentEvalResponse` contract from the radical brainstorm. The lens is the enforcement mechanism.
- Creates a natural boundary where Assay can measure compression ratios for both directions — useful for tuning and cost reporting.

### Scope

Medium — outbound extraction is primarily schema validation logic in assay-mcp. Inbound reuses the pipeline from Proposal 2. The schema enforcement is already partially designed (`AgentEvalResponse` type exists in the roadmap).

### Risks

- Outbound extraction assumes agents will produce semi-structured output. If an agent gives pure prose with no markers, extraction fails silently. Mitigation: provide clear tool descriptions that specify the expected format; fall back to full-text capture when extraction confidence is low.
- Over-constraining agent responses might miss nuanced feedback. An agent's "careful analysis" might contain useful context that schema extraction discards. Mitigation: preserve raw response alongside extracted fields; configurable `preserve_raw: bool`.
- Two compression systems (pipeline for inbound, schema extraction for outbound) could confuse users. Mitigation: unified "Assay Lens" branding; single configuration section.

---

## Proposal 4: Context-Pressure-Adaptive Compression

### What

Instead of static compression settings, Assay dynamically adjusts compression aggressiveness based on the consuming agent's remaining context window. The orchestrator tracks each agent's estimated context usage and signals the MCP server about context pressure:

```
Context remaining > 75%  →  Verbose mode (full output, raw evidence)
Context remaining 25-75% →  Standard mode (structured output, key evidence)
Context remaining < 25%  →  Crisis mode (pass/fail + one-line summary only)
Context remaining < 10%  →  Binary mode (pass/fail boolean only)
```

This is implemented as a header/parameter on MCP tool calls: `context_pressure: f32` (0.0 = empty context, 1.0 = full). The compression pipeline (Proposal 2) uses this to decide which stages activate. Early in a session, agents get rich context. Late in a session, they get compressed essentials.

The orchestrator is the natural owner of this signal because it manages sessions and can estimate context usage. For non-orchestrated usage (direct MCP), the agent itself can pass the signal, or Assay uses a default pressure curve.

### Why

- Context windows are not unlimited — they are consumed resources. Treating them as such is a paradigm shift from "compress everything uniformly."
- Adaptive compression preserves information when it's cheap to do so and aggressively compresses when the context is under pressure. This is strictly better than uniform compression.
- Creates a natural feedback loop: the orchestrator can redistribute budget from agents that are nearly done (low context remaining = they succeeded early) to agents that are struggling (high context = they've been iterating).
- Aligns with LLM reality: performance degrades as context fills. Compressing at high pressure isn't just about cost — it helps the agent focus on what matters.

### Scope

Large — requires context tracking in the orchestrator (future crate), a protocol extension for MCP, and compression pipeline integration. However, the protocol design can happen now and the implementation can be incremental.

### Risks

- Accurate context estimation is hard. Different models have different tokenizers, and tool results count differently than user messages. Mitigation: conservative estimates, model-specific profiles, or delegate to the agent to self-report.
- Abrupt transitions between compression levels could confuse agents. Mitigation: smooth interpolation curves, not hard thresholds; allow hysteresis.
- Adds complexity to every MCP interaction. Every tool call now carries pressure metadata. Mitigation: optional parameter with sensible default (assume `0.5` if not provided).

---

## Proposal 5: Compression as a Gate — The "Information Fidelity" Gate

### What

Turn compression itself into a quality gate. After compression occurs, Assay evaluates whether critical information survived the compression process. This is a meta-gate: it doesn't evaluate the code or the work — it evaluates whether the *representation of the results* is faithful enough for the agent to make correct decisions.

```toml
[[gate]]
name = "information-fidelity"
kind = "InformationFidelity"

[gate.fidelity]
# Required signals that must survive compression
required_signals = ["pass_fail", "failure_location", "error_message"]
# Maximum acceptable information loss
max_loss_ratio = 0.3
# Method: "structural" (check fields present) or "semantic" (AI comparison)
method = "structural"
```

Structural fidelity checks whether required fields are present and non-empty after compression. Semantic fidelity (agent-evaluated) compares the compressed output against the original and rates whether a reader could make the same decision from either version.

If the fidelity gate fails, the pipeline backs off compression aggressiveness and retries. This creates a self-correcting system: compression is aggressive by default, but backs off when it destroys critical information.

### Why

- Directly addresses the biggest risk of compression: losing information that changes decisions. Instead of hoping compression is safe, *verify* it.
- Perfectly fits Assay's dual-track model: structural fidelity is deterministic, semantic fidelity is agent-evaluated. Same architecture, applied to a new domain.
- Creates a measurable quality metric for compression itself. Over time, Assay can learn which compression strategies preserve fidelity for different output types.
- Self-correcting: if the fidelity gate fails, the system automatically reduces compression. No human tuning needed.
- Novel — no existing tool treats compression quality as a gate. This could be a differentiator for Assay.

### Scope

Medium — a new `GateKind` variant, fidelity checking logic in assay-core, and integration with the compression pipeline. Structural checking is straightforward; semantic checking reuses the agent-eval infrastructure.

### Risks

- Circular dependency: if the fidelity gate itself consumes tokens, compression hasn't saved anything. Mitigation: structural fidelity is zero-cost; semantic fidelity runs only on sample (e.g., every 10th result).
- Defining "required signals" is domain-specific. A test suite gate needs failure_location; a lint gate needs violation_type. Mitigation: default profiles per `GateKind`, user-overridable.
- Semantic fidelity requires an AI call, adding latency. Mitigation: optional, off by default, runs asynchronously.

---

## Proposal 6: Structured-First Output Design — "Compression by Design"

### What

Rather than compressing verbose output after the fact, design Assay's output formats to be inherently compact. Every MCP tool response and CLI output has two representations:

1. **Wire format:** Minimal structured JSON designed for machine consumption. No prose, no formatting, no redundancy. Fields are terse keys.
2. **Display format:** Human-readable rendering of the same data, generated only for CLI/TUI surfaces.

```json
// Wire format (what MCP agents receive):
{"p":true,"c":"cargo test","x":0,"d":42,"f":0,"t":12}

// Expanded (what it means):
// passed=true, cmd="cargo test", exit=0, duration=42ms, failures=0, total=12

// Display format (what CLI shows):
// ✓ cargo test (12 tests, 42ms)
```

The MCP server always returns wire format. The CLI always renders display format. They share the same underlying `GateResult` struct with `#[serde(rename)]` attributes for compact wire serialization.

Additionally, introduce "diff-mode" responses: when an agent calls `gate/run` multiple times for the same spec, subsequent calls return only what changed since the last call.

```json
// First call: full result
{"p":false,"f":3,"t":12,"failures":["test_a","test_b","test_c"]}

// Second call after agent fixes: diff only
{"p":false,"f":1,"t":12,"delta":{"fixed":["test_a","test_b"],"still_failing":["test_c"]}}
```

### Why

- "Don't compress — don't be verbose in the first place." This inverts the problem. Instead of RTK stripping ANSI codes from verbose output, Assay produces clean output natively.
- Single-character field names in wire format save 60-80% of tokens on field names alone for deeply-nested JSON — and this is zero-cost, pure serde attributes.
- Diff-mode eliminates the dominant cost in iterative agent workflows: re-sending results the agent already knows. An agent running gates 5-10 times during a coding session only pays full cost once.
- Clear separation between machine and human formats prevents the "pretty for who?" problem where output is formatted for humans but consumed by agents.
- Aligns with the schemars pipeline: JSON Schema documents both wire and display formats.

### Scope

Small-to-medium — serde rename attributes on types, diff calculation logic in assay-core, and a session-aware MCP handler for diff-mode. No new crates.

### Risks

- Single-character field names are unreadable during debugging. Mitigation: display format exists; wire format is documented via JSON Schema; a `?verbose=true` parameter on MCP tools returns expanded format for debugging.
- Diff-mode requires server-side session state (remember last result per spec per agent). Mitigation: in-memory HashMap keyed by agent session; stateless fallback returns full results.
- Diff logic could have bugs that cause agents to miss regressions (a test that passed, then failed again). Mitigation: diff always shows both fixed and regressed items; periodic full-refresh (every Nth call).

---

## Summary Matrix

| # | Proposal | Novelty | Scope | Depends On | Key Insight |
|---|----------|---------|-------|------------|-------------|
| 1 | Token Budgets as Config | Medium | Medium | assay-types, assay-core | Declare budgets, don't retroactively compress |
| 2 | Composable Middleware Pipeline | Medium | Large | assay-core | Dual-track pattern generalizes to compression |
| 3 | Bidirectional "Assay Lens" | High | Medium | assay-mcp | Compress both directions; schema-enforce outbound |
| 4 | Context-Pressure-Adaptive | High | Large | Orchestrator | Context is a depleting resource, not a dump |
| 5 | Compression as a Gate | High | Medium | Proposals 1+2 | Verify compression doesn't destroy decisions |
| 6 | Structured-First + Diff-Mode | Medium | Small-Med | assay-types, assay-mcp | Don't compress, don't be verbose in the first place |
