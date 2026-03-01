# Deterministic CLI/Tool Output Compression — Final Report

> Explorer: explorer-deterministic | Challenger: challenger-deterministic
> Date: 2026-02-28 / 2026-03-01
> Rounds: 3 (initial proposals → challenge → refinement → convergence)

---

## Executive Summary

Six proposals were explored for deterministic (non-AI) compression of Assay's gate output, MCP responses, and CLI results. After three rounds of debate, the key outcome is a **reframe**: instead of building separate compression modules, the brainstorm identified **design decisions that should be baked into Phases 3, 7, and 8** to avoid painting the architecture into a corner. Two ideas survive as deferred v0.2 features pending real data.

The core insight: Assay has domain knowledge that enables smarter compression than a generic proxy like RTK — it knows pass/fail semantics, gate structure, and consumer intent. But building compression infrastructure for a pipeline that doesn't exist yet (gate evaluation is Phase 7, MCP tools are Phase 8) would be premature. The right move is to design the pipeline with compression-friendly seams from day one.

---

## What Survived (Phase 3/7/8 Design Decisions)

These are constraints and design choices for implementers, not separate work items.

### 1. Streaming Capture with Exit-Code-Aware Byte Budget → Phase 7

**Decision:** Gate evaluation (`assay_core::gate::evaluate()`) should NOT use `Command::output()` which captures unbounded stdout/stderr into `Vec<u8>`. Instead, use streaming capture via `BufReader` with a configurable byte limit.

**Design:**
- **Passing gates (exit_code == 0):** Aggressive budget. Capture first N + last N lines within limit. Passing output is low-priority evidence.
- **Failing gates (exit_code != 0):** Conservative budget with error-marker preservation:
  1. Scan for common failure markers (`FAIL`, `ERROR`, `panicked`, `assertion failed`)
  2. Preserve all marker lines + surrounding context lines
  3. If markers + context exceed budget, truncate longest blocks but keep ALL markers
  4. Fall back to first N + last N if no markers found

**Limitation (document explicitly):** Marker scanning is best-effort, English-centric, not exhaustive. Custom test runners with non-standard markers degrade gracefully to first N + last N truncation within the failure budget. The exit code still signals failure regardless — markers help preserve the *why*.

**Configuration:**
```toml
[gate.defaults]
max_output_bytes = 32768  # ~8K tokens, configurable per-gate
```

**Rationale:** Prevents runaway memory allocation from verbose test suites (10K+ lines of stack traces). Designing for bounded capture from day one is cheaper than retrofitting. This is infrastructure, not premature optimization — `Command::output()` is a design choice that's hard to undo.

### 2. Truncation Metadata on GateResult → Phase 3

**Decision:** Add `truncated` and `original_bytes` fields to `GateResult` in the domain model.

```rust
pub struct GateResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub timestamp: String,
    pub truncated: bool,            // Was output budget-limited?
    pub original_bytes: Option<u64>, // Pre-truncation size (None if not truncated)
}
```

**Rationale:** The DTO captures **what happened** (evidence). Truncation is a fact about the evidence, not a presentation concern. The agent (or human) needs to know that output was clipped so it can request full output or re-run with a higher budget if needed.

**Explicitly NOT on GateResult:** A `summary: String` field. Summary computation ("3/4 passed") is a presentation concern that belongs on the MCP response type, not the evidence DTO. This respects the project convention: assay-types = pub DTOs with zero logic.

### 3. Summary-First MCP Response with Verbose Parameter → Phase 8

**Decision:** The `gate_run` MCP tool should return a summary-first response shape by default, with an optional `verbose` parameter for full evidence.

**Default response (summary-first):**
```json
{
  "summary": "3/4 criteria passed",
  "passed": false,
  "failures": [
    {
      "criterion": "clippy",
      "exit_code": 1,
      "stderr": "warning[E0599]: no method named...",
      "truncated": false
    }
  ],
  "passes": ["tests", "fmt", "build"]
}
```

**Verbose response (`gate_run(spec: "auth", verbose: true)`):**
```json
{
  "summary": "3/4 criteria passed",
  "passed": false,
  "results": [
    {"criterion": "tests", "passed": true, "stdout": "...", "stderr": "", ...},
    {"criterion": "clippy", "passed": false, "stderr": "...", ...},
    {"criterion": "fmt", "passed": true, "stdout": "", ...},
    {"criterion": "build", "passed": true, "stdout": "...", ...}
  ]
}
```

**Rationale:** The summary-first shape eliminates evidence for passing gates (the common case), reducing token consumption by 40-70% for typical gate runs. The agent sees pass/fail status immediately without iterating. The `verbose` parameter preserves access to passing evidence for cases where the agent needs to inspect *why* something passed (deprecation warnings, skipped tests, coverage drops) — without requiring a second round trip.

**Implementation note:** This is a view type (`GateRunResponse`) derived from `Vec<GateResult>`, not a replacement for GateResult. The core model stays clean; the MCP layer computes the summary.

### 4. Serde Hygiene on All Response Types → Phase 3/8

**Decision:** Apply `#[serde(skip_serializing_if = "...")]` annotations on GateResult and MCP response types for empty fields. Empty `stdout: ""`, `stderr: ""`, and `None` values should not appear in serialized JSON.

**Rationale:** Standard practice, not a feature. But worth documenting as a design constraint so implementers don't ship naive serialization with 30% overhead from empty fields.

---

## What's Deferred to v0.2 (Needs Real Data)

### 5. Evidence Compressor

**What:** A compression layer on GateResult stdout/stderr that applies:
- ANSI stripping (safe, always beneficial)
- Blank line / trailing whitespace collapse (safe)
- Duplicate identical line collapse with counts (safe for truly identical lines)
- Conditional pass elision (OFF by default — only for clean passes with empty stderr and no warning patterns)

**Why deferred:** Pass elision rules can't be validated without real gate output. Needs data on what passing output actually contains in practice. Could violate Assay's evidence-capture mandate if rules are too aggressive.

**Validation criteria for v0.2:** Measure actual token consumption of gate output in v0.1. If any individual gate consistently produces >2K tokens of passing output, build the compressor with data-driven elision rules.

### 6. Tool-Aware Parsers

**What:** Recognize common dev tool output (cargo test, clippy, eslint, jest) and apply tool-specific extraction of pass/fail counts + failure messages.

**Why deferred:** Maintenance burden (tool output formats change), dispatch fragility (matching command strings), no evidence that agents perform better with pre-digested output vs. raw output, and large scope (~6-8hrs) for unvalidated hypothesis.

**Validation criteria for v0.2:** Identify the top-3 token-burning gate tools by measuring real output sizes. Build parsers only for tools where compression savings exceed 70% and the parser can be kept under 100 lines.

---

## What Was Killed

### 7. Consumer-Aware Output Profiles (Idea 3)

**Killed reason:** Premature abstraction. A trait hierarchy with 4 implementations (`agent`, `human`, `structured`, `diff`) for zero existing consumers. The standard pattern: build concrete implementations, notice patterns, extract the trait. Not the reverse.

**What survives as an insight:** Different consumers want different verbosity. For v0.1, this is a boolean (`verbose: bool`), not a trait.

### 8. Cross-Run Delta Mode (Idea 6)

**Killed reason:** Three fatal problems:
1. Agent context eviction makes delta references unreliable — if run #1 output is compressed/evicted from the agent's context, "unchanged since run #1" is worse than useless
2. MCP session lifecycle is ambiguous — no clear reset semantics
3. Statefulness (HashMap on AssayServer) creates debugging nightmares

The fallback (return full output when baseline is lost) means complexity is added for a benefit that only materializes under perfect conditions.

---

## Relationship to RTK

Assay's compression operates at a different layer than RTK:
- **RTK:** Wraps CLI commands (git, cargo, npm) at the shell level. Compresses command-to-agent output. Single Rust binary, no domain knowledge.
- **Assay:** Compresses gate-evaluation-to-agent output inside the MCP pipeline. Has domain knowledge (pass/fail, criterion structure, evidence semantics).

They don't overlap. RTK can't compress MCP tool responses because it wraps shell commands, not MCP servers. An Assay user could use both: RTK for general CLI token savings, Assay's built-in compression for gate-specific token savings.

---

## Implementation Guidance

| Decision | Phase | Implementer Action |
|----------|-------|--------------------|
| Streaming capture with byte budget | Phase 7 | Use `BufReader` + bounded capture, not `Command::output()`. Exit-code-aware strategy. |
| Truncation metadata on GateResult | Phase 3 | Add `truncated: bool` + `original_bytes: Option<u64>` to the type |
| Summary-first MCP response | Phase 8 | Define `GateRunResponse` view type with summary/failures/passes shape. Add `verbose` param. |
| Serde skip_serializing_if | Phase 3/8 | Annotate all Option/String/Vec fields with skip conditions |
| Evidence compressor | v0.2 | Build after measuring real output sizes in v0.1 |
| Tool-aware parsers | v0.2 | Build after identifying top-3 token-burning tools |

---

## Estimated Token Savings

Conservative estimates based on RTK benchmarks and MCP response structure analysis:

| Technique | Applicable To | Estimated Savings | Confidence |
|-----------|---------------|-------------------|------------|
| Streaming budget enforcement | Gate stdout/stderr | 50-80% on verbose tools | High (bounded by design) |
| Summary-first MCP responses | gate_run results | 40-70% per response | High (structural, measurable) |
| Serde skip_serializing_if | All JSON responses | 10-30% | High (mechanical) |
| Evidence compressor (v0.2) | Gate stdout/stderr | 20-50% additional | Medium (depends on output) |
| Tool-aware parsers (v0.2) | Specific tools | 70-90% for matched tools | Low (unvalidated) |

**Combined v0.1 potential:** 50-80% token reduction on gate output reaching agents, with zero AI and zero external dependencies.

---

*Report finalized: 2026-03-01*
*Explorer: explorer-deterministic | Challenger: challenger-deterministic*
