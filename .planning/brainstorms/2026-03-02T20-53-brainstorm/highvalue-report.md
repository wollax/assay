# High-Value Features Report: Assay v0.2

**Explorer:** explorer-highvalue | **Challenger:** challenger-highvalue
**Date:** 2026-03-02
**Rounds:** 3 (initial proposals → challenger critique → explorer revision → convergence)

---

## Executive Summary

Six features were proposed, debated, and pressure-tested. Three survived for v0.2, one as a should-build, two dropped entirely, and three deferred to v0.3. The most significant outcome was rescoping the agent-evaluated gates feature from "Assay calls an LLM" to "Assay records agent-submitted evaluations via MCP" — dramatically reducing scope while preserving the category-defining differentiator.

---

## Agreed v0.2 Scope

### Must-Build #1: Run History (JSON Persistence)

**What:** Persist gate evaluation results to `.assay/results/<spec-slug>/<timestamp>.json` as append-only JSON files. Each run is recorded with: timestamp, spec slug, per-criterion results, stdout/stderr evidence, duration, and pass/fail status. Add CLI command `assay history <spec>` to list recent runs.

**Why (consensus):** Originally proposed as a "should-build," the challenger elevated this to must-build #1. Key argument: you cannot iterate on agent gate quality without persistence. If agent-evaluated gates (must-build #3) ship without history, results vanish after each run — making it impossible to detect evaluation drift, debug flaky agent judgments, or build human trust. History is infrastructure that unblocks everything downstream.

**Scope:** ~200-300 lines. `GateRunRecord` struct, file I/O, one new CLI subcommand. Low risk, no new dependencies.

**Design notes:**
- One JSON file per run, not one file per spec (avoids concurrent write issues)
- `.assay/results/` should be `.gitignore`d by default
- Evidence (stdout/stderr) subject to existing 64KB truncation cap
- No SQLite, no indexing, no query engine in v0.2. Just structured persistence.

### Must-Build #2: Required/Advisory Gates

**What:** Add an enforcement level to `Criterion` that distinguishes between criteria that must pass (required) and criteria that provide feedback without blocking (advisory). `GateRunSummary` reports required vs. advisory failures separately. Pass/fail logic: all required criteria must pass for the gate to pass; advisory failures are warnings.

**Why (consensus):** This is table-stakes for the dual-track quality model. Real quality policies need "tests must pass" (required) alongside "style guidelines" (advisory). Without this distinction, all criteria are equally blocking, which makes users either over-constrain (every nitpick blocks) or under-constrain (leave out advisory quality signals).

**Scope:** One new field on `Criterion`, changed summary semantics in `evaluate_all()`. Low complexity.

**Open design question (for team lead):**

Option A — Boolean: `optional: bool` (default false). Simple, YAGNI-compliant.
```toml
[[criteria]]
name = "style-check"
optional = true
```

Option B — Enum: `enforcement: "required" | "advisory"` (default "required"). Same complexity, better readability.
```toml
[[criteria]]
name = "style-check"
enforcement = "advisory"
```

Explorer argues for the enum (communicates intent over mechanism). Challenger leans boolean (YAGNI). Both agree it's a minor design call.

**Implementation note:** `Criterion` has `#[serde(deny_unknown_fields)]`. Adding a new field is backward-compatible (existing specs without the field get the default value), but any users with experimental/unknown fields in their specs will break. This is by design — strict schemas — but worth noting in release notes.

### Must-Build #3: Agent Gate Recording (`gate_report` MCP Tool)

**What:** Add a `gate_report` MCP tool that allows an external agent to submit a pass/fail evaluation for a specific criterion, with structured reasoning. Assay records the result as `GateKind::AgentReported` and persists it to Run History. The agent provides: spec name, criterion name, passed (bool), reasoning (string), and evaluator_role ("self", "independent", or "human").

**Why (consensus, after significant debate):**

This is Assay's category-defining feature — quality criteria verified by more than exit codes. The debate centered on HOW to deliver it:

- **Explorer's original proposal:** Assay calls an LLM directly (HTTP client, API key management, retry/backoff, rate limiting). Three phases of increasing sophistication.
- **Challenger's counter:** Invert the dependency — Assay RECEIVES evaluations from agents already running, rather than making its own LLM calls. No HTTP client, no API keys, dramatically simpler.
- **Explorer's pushback:** Self-evaluation has a trust problem — "student grading their own exam." The implementing agent will be biased toward passing its own work.
- **Challenger's resolution:** The trust problem is real but unsolvable in v0.2 without orchestration (spawning independent evaluator = multi-agent coordination = v0.3). For v0.2: (a) structured per-criterion evaluation adds rigor even with self-evaluation, (b) Run History creates an audit trail for human review, (c) the `evaluator_role` metadata field distinguishes self-evaluated from independently-evaluated results for future trust analysis.

**Final agreement:** Ship `gate_report` with self-evaluation in v0.2. The recording mechanism is agent-agnostic — when the orchestrator arrives in v0.3, swapping in an independent evaluator requires zero changes to the MCP tool.

**Scope:**
- New `GateKind::AgentReported` variant in `assay-types`
- New `gate_report` MCP tool in `assay-mcp` (receives pre-computed results, no async execution needed)
- Flat schema: `{ spec_name, criterion_name, passed, reasoning, evaluator_role }`
- Results persisted via Run History (#1)
- ~300-400 lines across types, MCP server, and persistence

**Trust model evolution roadmap:**
| Version | Evaluator | Trust mechanism |
|---------|-----------|----------------|
| v0.2 | Self (implementing agent) | Human audit via Run History |
| v0.3 | Independent (separate agent via orchestrator) | Architectural separation |
| v0.3+ | Assay-controlled (context assembly) | Assay strips implementation context, provides only output artifacts |
| v0.4+ | Built-in LLM (optional) | For CI/headless environments without a running agent |

### Should-Build: Wire `FileExists` into `evaluate()`

**What:** Connect the existing `evaluate_file_exists()` function (currently dead code in `gate/mod.rs`) into the `evaluate()` dispatch path. Requires adding a `path` field to `Criterion`.

**Why:** It's already implemented but unreachable. Small scope, completes existing work.

---

## Deferred to v0.3

### Context-Controlled Agent Evaluation (`gate_evaluate`)

**What:** MCP tool where Assay assembles evaluation context (git diffs, file contents, spec metadata) and provides it to the calling agent for evaluation. Unlike `gate_report` (agent submits pre-computed result), `gate_evaluate` has Assay control what context the evaluator sees.

**Why deferred:** Context assembly (git diffs, selective file inclusion, token budgeting) is non-trivial. But this is where the real differentiation lives — by controlling context, Assay can strip implementation rationale (fixing self-serving bias) and provide only output artifacts (fixing context blindness).

### Worktree Orchestrator

**What:** Create worktree → launch agent → run gates → report. The atomic "one spec, one agent, one worktree" loop.

**Why deferred:** Requires agent launcher abstraction (hardest design question: what IS an agent?), git worktree management, and the CWD threading problem (all domain functions currently derive paths from `std::env::current_dir()`). This is the foundation for v0.3 but has significant design surface.

**Key design issue identified during debate:** The agent launcher design is the crux. Different AI coding agents (Claude Code, Cursor, Aider) have different invocation models, context injection, and completion signals. Start designing the trait now even though implementation is v0.3.

### TUI Dashboard

**What:** Replace TUI skeleton with functional spec/gate viewer, then session monitor.

**Why deferred:** The primary interaction surface is the agent plugin, not a TUI. The TUI's value materializes with concurrent multi-spec orchestration (v0.3+). Until then, CLI output is sufficient.

### Independent Evaluator Enforcement

**What:** Ensure the agent evaluating quality is NOT the agent that implemented the code.

**Why deferred:** Requires multi-agent coordination (orchestrator). The `evaluator_role` metadata in v0.2 lays the groundwork.

---

## Dropped (Not Planned for Any Version)

### SpecProvider Trait

**Original proposal:** Abstract spec loading behind a trait for pluggable sources (Kata, Linear, Jira).

**Why dropped:** A trait with one implementation is premature abstraction. `spec::scan()` already takes `&Path`. Wait for a concrete second provider to materialize before abstracting. Per project conventions: "don't design for hypothetical future requirements."

### Trust Calibration / Consensus Mode

**Original proposal:** Confidence thresholds, multi-evaluator consensus, human-in-the-loop for low-confidence judgments.

**Why dropped:** LLM confidence scores are not well-calibrated. This is a research problem, not an engineering problem. Consensus mode (multiple LLM calls) multiplies cost and latency without proven reliability benefits. Defer indefinitely.

### Composite Gate Logic (AND/OR/Threshold)

**Original proposal:** Boolean composition of gates (all must pass, any must pass, N of M).

**Why dropped:** The required/advisory distinction delivers the meaningful quality policy expressiveness. Full boolean composition adds spec format complexity (TOML doesn't naturally express deeply nested structures), new evaluation models, and new error states — without demonstrated user demand.

### Async Pipeline Bifurcation

**Original proposal:** Prep the evaluation engine for async by creating separate sync/async evaluation paths.

**Why dropped:** `gate_report` receives pre-computed results — no async execution needed. The only consumer of async evaluation is the built-in LLM evaluator (v0.4+), which may never ship. Don't prep infrastructure for something without a consumer.

### Built-in LLM Client

**Original proposal:** Assay calls LLM APIs directly for agent-evaluated gates.

**Why dropped (for now):** The `gate_report` → `gate_evaluate` progression may make this unnecessary. Agents already have LLM access. Building a second LLM client inside Assay adds HTTP client dependency, API key management, retry/backoff, rate limiting, and provider-specific code — significant complexity with uncertain value. Revisit in v0.4+ only if headless/CI use cases demand it.

---

## Key Insights from the Debate

1. **Sequence infrastructure before differentiators.** Run History is boring but unblocks agent gate quality iteration. Without persistence, agent evaluations are fire-and-forget experiments you can't learn from.

2. **Invert dependencies to reduce scope.** Instead of Assay building an LLM subsystem to evaluate gates, let running agents submit evaluations via MCP. Same differentiator, fraction of the complexity.

3. **The trust problem is real but has a pragmatic v0.2 answer.** Self-evaluation with structural rigor + human audit trail is imperfect but shippable. Architectural separation (independent evaluator) is the correct v0.3 solution, not algorithmic trust (confidence scores).

4. **Beware proposals that smuggle larger features.** "Trigger a second agent to evaluate" sounds like a gate feature but is actually orchestration. Name the dependency explicitly.

5. **Readability over mechanism for user-facing config.** `enforcement = "advisory"` communicates intent; `optional = true` communicates mechanism. For a quality standards tool, intent matters.

---

## Estimated v0.2 Total Scope

| Feature | Lines (est.) | Risk | Dependencies |
|---------|-------------|------|-------------|
| Run History | 200-300 | Low | None |
| Required/Advisory | 100-150 | Low | Schema bump |
| `gate_report` MCP tool | 300-400 | Medium | Run History |
| Wire `FileExists` | 50-100 | Low | None |
| **Total** | **650-950** | **Low-Medium** | **Sequential** |

Build order: History → Required/Advisory → `gate_report` → FileExists. Each feature can ship independently, but `gate_report` depends on History for persistence.
