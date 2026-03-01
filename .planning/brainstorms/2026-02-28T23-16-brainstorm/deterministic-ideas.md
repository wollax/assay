# Deterministic CLI/Tool Output Compression — Explorer Ideas

> Explorer: explorer-deterministic | Date: 2026-02-28
> Context: RTK-like deterministic filtering applied to Assay's gate output, MCP responses, and CLI results. Zero AI, pure pattern matching and structural compression.

## The Compression Opportunity

Assay sits at a unique intersection: it **orchestrates** commands (gate evaluation via `std::process::Command`) and **exposes results** to AI agents (via MCP `gate_run` tool responses). Both surfaces produce verbose output that burns tokens:

- `cargo test` on a 200-test project: ~5,000 tokens of output. Agent only needs: "198 passed, 2 failed" + the 2 failure traces.
- `cargo clippy`: Hundreds of lines of warnings. Agent needs: unique warning categories + counts + first occurrence of each.
- `GateResult` JSON over MCP: Empty `stdout`/`stderr` fields still serialize. Passing gates carry evidence nobody reads.

RTK proves this pattern works at 60-90% savings. Assay can do **better** than RTK because it has **domain knowledge** — it knows what a gate is, what passing vs. failing means, and what the consuming agent actually needs.

---

## Idea 1: GateResult Evidence Compressor

**Name:** Evidence Compressor

**What:** A configurable compression layer applied to `GateResult.stdout` and `GateResult.stderr` before they reach the consumer (CLI display or MCP JSON response). The compressor operates on raw strings and applies:

1. **ANSI stripping** — Remove color codes, cursor control sequences. Agents can't render them; humans see them in terminal, not in JSON.
2. **Trailing whitespace / empty line collapse** — 20 blank lines become 0.
3. **Pass elision** — For passing gates (`exit_code == 0`), replace stdout with a one-line summary: `"198 tests passed in 2.3s"` (if parseable) or truncate to first N lines + `"... (truncated, {total} lines)"`.
4. **Failure focus** — For failing gates, keep the **full stderr** and the **last N lines of stdout** (where most test frameworks report failures). Prepend a summary: `"2 of 200 tests failed"`.
5. **Duplicate line collapse** — Repeated identical lines (common in build output) compressed to `"{line} (repeated {N} times)"`.

The compressor is a pure function: `fn compress_evidence(result: &GateResult, config: &CompressionConfig) -> CompressedEvidence`. It sits between gate evaluation and result serialization, so both CLI and MCP benefit.

**Why:** This is the highest-impact, lowest-risk compression. GateResult evidence is the single largest token consumer in Assay's output. RTK achieves 60-90% on similar output. Assay can be smarter because it has the `exit_code` context — it knows whether the output is a success or failure log, and can compress accordingly.

**Scope:** Medium. ~3-4 hours. Regex-based ANSI stripping, line counting, configurable thresholds. Most complexity is in testing edge cases (multi-byte output, binary garbage, extremely long lines).

**Risks:**
- Over-compression hides useful debug info from the agent. Mitigation: always include the uncompressed byte count so the agent can request the full output if needed.
- Regex performance on large outputs. Mitigation: cap input at a configurable byte limit (e.g., 1MB) and truncate before processing.
- Different tools have different output formats — a generic compressor might mangle structured output (JSON, XML). Mitigation: detect structured output formats and skip compression, or use format-specific handlers (see Idea 4).

---

## Idea 2: MCP Response Envelope Optimization

**Name:** Lean MCP Responses

**What:** Optimize the JSON structure of MCP tool responses to minimize token consumption while preserving all semantics. Three techniques:

1. **Omit empty/default fields** — `serde(skip_serializing_if = "...")` on GateResult fields. Empty `stdout: ""` and `stderr: ""` should not appear in JSON. `exit_code: Some(0)` for passing gates is redundant with `passed: true`.
2. **Summary-first response format** — For `gate_run`, return a response shaped like:
   ```json
   {
     "summary": "4/5 criteria passed",
     "passed": false,
     "failures": [{ "criterion": "clippy", "stderr": "..." }],
     "passes": ["tests", "fmt", "build", "docs"]
   }
   ```
   Instead of the naive `Vec<GateResult>` which repeats all fields for all results, the summary-first format puts what the agent needs most (did it pass? what failed?) at the top and omits evidence for passing gates entirely.
3. **Spec reference compression** — Instead of embedding the full spec in every gate_run response, return only the spec name and version hash. The agent already has the spec from `spec_get`.

**Why:** MCP responses are consumed by token-limited models. Every unnecessary JSON key, empty string, and redundant field costs tokens. This optimization is invisible to the calling agent (same semantic content) but can reduce response size by 40-70% for typical gate runs. The summary-first format also makes the response more useful — the agent can pattern-match on `"passed": true` without parsing individual results.

**Scope:** Small-Medium. ~2-3 hours. Mostly `#[serde(skip_serializing_if)]` annotations + a new summary response type. The summary type is a separate concern from GateResult and doesn't change the core domain model.

**Risks:**
- Changing MCP response shape breaks clients that depend on the current format. Mitigation: this is pre-v0.1 — there are no clients yet. Also, MCP tool responses are opaque text/JSON; the agent adapts to whatever schema the tool description advertises.
- Omitting fields makes responses harder to debug. Mitigation: a `verbose=true` parameter on `gate_run` that bypasses compression for debugging.
- Summary-first is a new response type alongside GateResult, adding type complexity. Mitigation: it's a view type derived from `Vec<GateResult>`, not a replacement. The core model stays clean.

---

## Idea 3: Output Format Profiles

**Name:** Consumer-Aware Formatting

**What:** Define compression profiles that tailor output verbosity to the consumer:

| Profile | Consumer | Behavior |
|---------|----------|----------|
| `agent` | MCP tool responses | Maximum compression. Summary-first JSON. Evidence only for failures. No ANSI. |
| `human` | CLI terminal | Moderate compression. ANSI preserved. Pass/fail summary + failure details. |
| `structured` | CI/CD, logging, TUI | No compression. Full GateResult with all fields. Machine-parseable. |
| `diff` | Repeat gate runs | Show only what changed since last run. Zero output for unchanged results. |

The profile is selected automatically based on context (MCP handler → `agent`, CLI → `human`, `--format json` → `structured`) or overridden by configuration.

The implementation is a trait:
```rust
trait OutputFormatter {
    fn format_gate_results(&self, results: &[GateResult]) -> FormattedOutput;
}
```

Each profile implements the trait. The gate evaluation pipeline takes a `&dyn OutputFormatter` and applies it at the boundary.

**Why:** This is the architectural lever that makes all other compression ideas composable. Without profiles, you'd hardcode compression rules and break one consumer to optimize for another. With profiles, the MCP layer gets aggressive compression while the TUI gets full fidelity — from the same gate evaluation. RTK essentially has one profile (CLI agent). Assay has four consumers and needs to serve them all.

**Scope:** Medium. ~3-4 hours. Trait definition, 3-4 implementations, integration into gate evaluation pipeline. The `diff` profile is the most complex (requires session state) and could be deferred.

**Risks:**
- Profile proliferation — too many knobs. Mitigation: start with `agent` and `human` only. Add others when a consumer demands them.
- Abstraction overhead for a v0.1 project. Counter: the trait is 1 method. The concrete implementations are functions. This is minimal abstraction that prevents hardcoding.
- `diff` profile requires state (previous run results). This drags in persistence, which is explicitly out of scope. Mitigation: defer `diff` to v0.2 and document the concept.

---

## Idea 4: Gate-Specific Output Parsers

**Name:** Tool-Aware Compression

**What:** Recognize common development tool output formats and apply tool-specific compression. RTK does this with command-specific filters for `git`, `cargo`, `npm`, etc. Assay can do the same for gate output:

| Tool Pattern | Recognition | Compression |
|---|---|---|
| `cargo test` | `running N tests` + `test result:` lines | Extract pass/fail counts, keep only failing test names + output |
| `cargo clippy` | `warning[EXXXX]:` headers | Group by warning code, count occurrences, show first instance of each |
| `cargo build` | `Compiling ... (N of M)` progress lines | Collapse to `"Built N crates in Xs"` |
| `eslint` / `biome` | JSON reporter output | Parse JSON, count by severity, show top-N |
| `jest` / `vitest` | Test summary block | Extract summary line, keep failure traces |
| Generic | Unrecognized format | Apply default compression (Idea 1) |

The parser is selected by matching the gate command string against patterns (`cmd.starts_with("cargo test")`, regex on command structure). If no specific parser matches, the generic compressor (Idea 1) applies.

Each parser is a function: `fn compress_cargo_test(stdout: &str, stderr: &str) -> CompressedEvidence`.

**Why:** Generic compression leaves tokens on the table. `cargo test` output has a very predictable structure — you can extract exactly what the agent needs (pass count, fail count, failure messages) and discard everything else. This is where RTK gets its highest savings (70-90% on test output). Assay knows the gate command and can route to the right parser.

**Scope:** Large. ~6-8 hours for 4-5 parsers + the dispatch logic. Each parser is independent, so they can be added incrementally. Start with `cargo test` and `cargo clippy` (highest value for a Rust project), add others as needed.

**Risks:**
- Fragile pattern matching — tool output formats change between versions. Mitigation: parsers fall back to generic compression on parse failure, so a format change degrades gracefully rather than breaking.
- Scope creep — every tool is a rabbit hole. Mitigation: strict parser contract (input: raw strings, output: CompressedEvidence). Each parser is a self-contained function. Ship 2, document the extension pattern for community contribution.
- Testing burden — each parser needs test fixtures with real tool output. Mitigation: snapshot tests with captured real output. This is actually a strength — the fixtures document the expected format.

---

## Idea 5: Streaming Truncation with Budget

**Name:** Token Budget Enforcement

**What:** Apply a **token budget** to gate output at the point of capture. Instead of capturing all stdout/stderr and compressing after the fact, stream the output through a budget-aware filter that stops capturing once a configurable token limit is reached.

The budget is configured per-gate or globally:
```toml
[gate.defaults]
max_output_tokens = 2000  # ~8KB of text

[gate.overrides."cargo test"]
max_output_tokens = 4000  # Tests get more budget
```

The filter prioritizes content:
1. First N lines (command preamble — often contains version info, config summary)
2. Last N lines (most tools put summaries at the end)
3. Middle content only if budget remains

When truncated, the output includes a marker:
```
... [{N} lines truncated, {total} total, budget {budget} tokens] ...
```

This is applied at the `std::process::Command` output capture level, before `GateResult` construction.

**Why:** All other compression ideas operate on already-captured output. If a test suite dumps 100,000 lines of trace logging to stdout, you've already allocated memory for it and paid the I/O cost. Token budget enforcement at the capture layer prevents the problem upstream. This is especially important for Assay's orchestration layer where N agents run gates concurrently — memory matters.

**Scope:** Medium. ~3-4 hours. BufReader-based streaming capture with line counting and budget tracking. The priority capture (first N + last N) requires buffering the last N lines, which is a bounded ring buffer.

**Risks:**
- Truncation destroys evidence needed for debugging. Mitigation: the full output is still available via a `--verbose` flag or by re-running the gate. The truncated output includes byte counts so the agent knows content was lost.
- Token estimation from byte count is approximate (UTF-8, different tokenizers). Mitigation: use a simple heuristic (4 chars ≈ 1 token for English text) and be conservative. Exact tokenization is overkill.
- Ring buffer for "last N lines" adds complexity to the capture path. Mitigation: use a `VecDeque` with a fixed capacity. This is ~20 lines of code.

---

## Idea 6: Idempotent Output Deduplication Across Sessions

**Name:** Cross-Run Delta Mode

**What:** When an agent runs `gate_run` multiple times during a session (common pattern: implement → test → fix → test → fix → test), subsequent runs produce mostly identical output. Delta mode detects this and returns only what changed:

1. **Content hash per criterion** — SHA-256 of `(stdout, stderr, exit_code)`. Stored in session memory (in-process hashmap, not persisted).
2. **First run** — Full output returned for all criteria.
3. **Subsequent runs** — Only changed criteria return full output. Unchanged criteria return: `"unchanged since run #{N}"` with their previous pass/fail status.
4. **Transition detection** — Special attention to criteria that changed status (pass→fail or fail→pass), which always include full output.

The session state lives in the `AssayServer` struct (for MCP) or a process-level cache (for CLI). It's ephemeral — dies with the process.

**Why:** This is the only compression technique that gets better over time within a session. The first run saves nothing, but the 5th run of a 20-criterion spec might compress 90%+ if the agent is iterating on a single failing test. This mirrors how a human developer reads test output — you stop reading the passing tests after the first run and focus on what changed.

**Scope:** Medium. ~3-4 hours. HashMap + SHA-256 per criterion. The tricky part is session identity — MCP connections don't have explicit session IDs, but the `AssayServer` instance lifetime serves as an implicit session.

**Risks:**
- Stale hashes — if the gate command has side effects (writes files, modifies env), the output might change semantically even if the byte content is identical. Mitigation: for v0.1, this is unlikely — gates are read-only evaluations. Flag as a known limitation.
- Memory growth — if criteria produce large outputs and the hash map stores full content for comparison, memory could grow. Mitigation: store only the hash + summary, not the full content. Re-capture and compare hashes, not content.
- MCP session semantics — a reconnecting client gets a fresh server instance (no session memory). This is correct behavior: delta mode is per-connection, and a new connection should start fresh.

---

## Cross-Cutting Observations

### Where These Ideas Fit in the Architecture

```
std::process::Command
        │
        ▼
  [Idea 5: Token Budget] ← capture layer, limits raw input
        │
        ▼
   Raw stdout/stderr
        │
        ▼
  [Idea 1: Evidence Compressor] ← generic string compression
        │
        ▼
  [Idea 4: Tool-Aware Parser] ← recognizes tool format, extracts semantics
        │
        ▼
   GateResult { stdout, stderr, ... }
        │
        ├──→ [Idea 2: MCP Envelope] ← JSON structure optimization
        │         │
        │         ▼
        │    MCP tool response (agent consumer)
        │
        ├──→ [Idea 3: Human Profile] ← terminal formatting
        │         │
        │         ▼
        │    CLI output (human consumer)
        │
        └──→ [Idea 6: Delta Mode] ← cross-run deduplication
                  │
                  ▼
             Subsequent MCP/CLI responses
```

### Priority Ranking

1. **Idea 1 (Evidence Compressor)** — Highest ROI. Generic, always applicable, directly reduces the biggest token sink.
2. **Idea 2 (Lean MCP Responses)** — Low effort, high impact. Serde annotations + summary type.
3. **Idea 5 (Token Budget)** — Infrastructure play. Prevents runaway output at the source.
4. **Idea 3 (Output Profiles)** — Architectural enabler. Makes other ideas composable. But adds abstraction early.
5. **Idea 4 (Tool-Aware Parsers)** — Highest compression potential but largest scope. Ship after generic compressor proves the pattern.
6. **Idea 6 (Delta Mode)** — Clever but requires session state. Best as a v0.2 feature after MCP sessions are stable.

### Relationship to RTK

RTK is a standalone binary that wraps existing commands. Assay's compression is **integrated** — it sits inside the gate evaluation pipeline, has domain context (pass/fail, criterion names), and serves multiple consumers. This means:
- Assay can compress more aggressively (it knows what "passing" means)
- Assay can't use RTK as a dependency (RTK wraps commands; Assay wraps results)
- But Assay can borrow RTK's patterns: regex-based ANSI stripping, line grouping, trailing whitespace collapse
