# High-Value Features — Consolidated Report
## Assay v0.4.0 Brainstorm

*Explorer: explorer-highvalue | Challenger: challenger-highvalue*
*3 rounds of debate | Date: 2026-03-10*

---

## Executive Summary

Seven features were proposed and pressure-tested over three rounds of debate. Four are recommended
for v0.4.0 core, two are deferred to v0.4.1 as extensions, and one (streaming) is hard-deferred
to v0.5.0. The v0.4.0 core delivers the headline capability: diff-aware headless gate evaluation
in a single MCP tool call.

---

## v0.4.0 Core (~4.5 weeks)

### Feature 1: `spec_validate` — Spec Health Checker
*Ships first, no dependencies | Estimate: 3 days*

**What**: New MCP tool `spec_validate(spec_name?, check_commands?: bool)` that statically validates
specs without running them. Returns structured `ValidationResult` with per-criterion diagnostics.

**Validation checks:**
- TOML parse errors with source-location context
- Criterion name uniqueness within a spec
- AgentReport criteria have non-empty `prompt` field (the field that drives evaluator agent behavior)
- Directory-based spec structure completeness (feature_spec.md present when declared)
- Cross-reference: if a spec declares `depends = [...]`, referenced specs exist
- Command existence on `$PATH` (opt-in via `check_commands: true` — off by default because
  the runtime environment may differ from the validation environment)

**Why first**: Eliminates wasted agent token burns on malformed specs. Pre-run validation lets
CI catch structural errors before any subprocess is spawned. Highest ROI per day of any feature.

**Key decisions from debate:**
- `check_commands` is opt-in (not default) to avoid false positives in containers/CI
- Validates `prompt` not `description` on AgentReport criteria — `prompt` is what the evaluating
  agent receives; `description` is human-facing metadata
- Cross-spec dependency validation needs cycle detection (not just existence check)

---

### Feature 2: Context Engine Workspace Crate
*Parallel with WorkSession | Estimate: 1 week*

**What**: Extract and formalize token estimation + context windowing into a new workspace crate
`assay-context` within the assay monorepo. The crate provides:
- `ContextWindow::from_budget(tokens: u32) -> ContextWindow` — priority-ordered content budget
- `ContextWindow::add_source(label, content, priority)` — registers content blocks
- `ContextWindow::build() -> String` — resolves what fits in the budget
- `TokenEstimator` trait with byte-heuristic default impl (no external dep)
- Optional `tiktoken-rs` feature flag for precision (not the default — crate is unmaintained)

**Why workspace crate, not separate repo**: Path deps are fragile, git deps require SHA
coordination, crates.io requires stable API before the design is proven. Workspace crate shares
via workspace dependency. Extract to separate repo only when Smelt actually needs to consume it
and the API has stabilized.

**Why needed for v0.4.0**: `gate_evaluate` must truncate the diff before feeding it to the
evaluator agent. Without principled budget allocation, truncation is arbitrary. The context
engine provides a consistent truncation strategy: diff gets a token budget computed from
(model context window − spec criteria tokens − system prompt tokens), and the budget is
filled head-first with tail fallback.

**Key decisions from debate:**
- Workspace crate, not separate repo — defer extraction until Smelt actually needs it
- Byte-heuristic fallback as default; `tiktoken-rs` behind optional feature flag
- `assay-context` name within the assay workspace

---

### Feature 3: `WorkSession` — Session Record Persistence
*Parallel with context engine | Estimate: 2 weeks*

**What**: New `WorkSession` type persisted as JSON under `.assay/sessions/<session-id>.json`.
Links: worktree path, spec name, agent invocation record, and one or more `GateRunRecord`
references (via `run_id`). Tracks phase transitions with timestamps:
`created → agent_running → gate_evaluated → completed | abandoned`

**MCP tools** (3 at launch, `session_get` deferred):
- `session_create(spec_name, worktree_path?)` — creates and persists new session
- `session_update(session_id, status, gate_run_id?)` — transitions phase, links gate runs
- `session_list(spec_name?, status?)` — enumerates sessions for Smelt orchestration

**Recovery logic on startup**: Scan `.assay/sessions/` for `agent_running` sessions on process
start. Sessions older than 24h → mark `abandoned`. Newer sessions → mark `abandoned` with
`recovery_note: "MCP server restarted"`. This is conservative but safe — better to mark
abandoned and let Smelt retry than to assume a hung session is still valid.

**Key architectural constraint from debate**: `gate_evaluate` calls session management as
**direct Rust function calls** within the same process — not MCP round-trips. The MCP
`session_*` tools are exclusively for external orchestration (Smelt, user queries). This
must be explicit in the implementation plan to prevent an MCP round-trip being added "for
consistency."

**Key decisions from debate:**
- 2 weeks (not 1) — recovery logic on startup is real work
- 3 MCP tools at launch (not 4) — `session_get` is 30-min addition after persistence layer exists
- `AgentSession` (in-memory, v0.3.0) and `WorkSession` (on-disk, v0.4.0) are distinct types
  for distinct purposes — do not conflate

---

### Feature 4: `gate_evaluate` — Diff-Aware Headless Evaluation Capstone
*Depends on WorkSession + context engine | Estimate: 2.5 weeks*

**What**: Single MCP tool `gate_evaluate(spec_name, base_ref?)` that computes a `git diff`,
spawns a headless Claude Code agent in `--print --output-format json` mode, collects
structured per-criterion evaluations from the output, and persists a complete `GateRunRecord`
in a single round-trip. Returns `run_id + pass/fail summary`.

**Execution model** (clarified in debate — the original proposal was misleading):

```
gate_evaluate(spec_name, base_ref = "main"):
  1. git diff <base_ref>  →  diff string
  2. load spec criteria (AgentReport criteria only — command criteria run separately)
  3. compute token budget for diff via context engine
  4. truncate diff to budget (head-first + tail fallback)
  5. spawn: claude --print --output-format json \
       --prompt "<system: spec criteria + evaluation instructions>\n<diff>"
  6. parse JSON output: [{criterion, passed, rationale, confidence}]
     → lenient: serde_json::Value intermediate parse + field extraction
     → handle extra fields, missing optional fields, partial responses
  7. call session finalization as Rust function (not MCP tool)
  8. return run_id + summary
```

The evaluator subprocess **does not call `gate_report` or `gate_finalize`**. Those MCP tools
are inaccessible to a subprocess on the same stdio transport. The parent process parses the
structured JSON output and finalizes internally. `--allowedTools` restriction is irrelevant
to this design.

**EvaluatorOutput schema** (define before prompt engineering, not after):
```json
{
  "evaluations": [
    {
      "criterion": "string (exact criterion name from spec)",
      "passed": "boolean",
      "rationale": "string (required, max ~200 words)",
      "confidence": "high | medium | low (optional)"
    }
  ],
  "overall_notes": "string (optional)"
}
```

Prompt engineering is anchored to this schema. Lenient deserialization handles variations.

**Why this is the headline feature**: No MCP tool in the ecosystem currently does diff-aware
AI gate evaluation in a single call. Today's workflow requires 4 manual steps (diff, gate_run,
repeated gate_report, gate_finalize). `gate_evaluate` collapses this and enables Smelt to
evaluate completed worktrees without ceremony.

**Key decisions from debate:**
- Option 2 subprocess model (parse structured JSON) — not Option 1 (second MCP transport)
- Define `EvaluatorOutput` schema first, then prompt engineering (not vice versa)
- Lenient `serde_json::Value` intermediate parse for robustness at 99% not 80%
- 2.5 weeks (not 2 or 3) — subprocess IO is easy, prompt engineering calibration is the variable

---

## v0.4.1 Extensions

### Feature 5: Gate DAG — Criteria Dependency Chains
*Opportunistic: pull into v0.4.0 core if evaluator refactor is ahead of schedule | Estimate: 4 days*

**What**: Allow spec criteria to declare `depends_on = ["criterion-a"]`. Assay builds a
dependency DAG, evaluates in topological order, and skips downstream criteria when an upstream
Required criterion fails.

**Schema design** (from debate — no new `GateResult` fields):
- Add `depends_on: Option<Vec<String>>` to `Criterion` in `assay-types` (new spec field)
- Add `skip_reason: Option<String>` to `CriterionResult` with `#[serde(default)]`
  (backwards compatible — old records default to `None`)
- Skipped criteria have `result: None, skip_reason: Some("upstream criterion 'build' failed")`
- Counting stays unchanged: `result: None` → skipped, regardless of `skip_reason`
- Disambiguation: `result: None, skip_reason: None` = pending; `result: None, skip_reason: Some(...)` = DAG-skipped

**Why NOT `GateKind::Skipped`**: Would cause `passed: false` to be counted as `required_failed`
in enforcement summaries, inflating failure counts and potentially blocking decisions incorrectly.
The `skip_reason` on `CriterionResult` is at the right abstraction level.

**Cycle detection**: Required. Naive cycles would deadlock the topological sort.

**Why opportunistic for v0.4.0**: The gate evaluator is already being refactored for
`gate_evaluate`. The topological sort is ~2 days of incremental work on top of that refactor.
If the milestone is running on schedule at the evaluator refactor point, Gate DAG pulls in
at low marginal cost. If behind schedule, it stays in v0.4.1 with no dependency impact.

---

### Feature 6: Criterion-Level Retry
*Independent — can ship any sprint | Estimate: 4 days*

**What**: Add `retry.max_attempts: u32` to individual gate criteria and `[gate]` config defaults.
The gate evaluator attempts the command up to `max_attempts` times. A criterion fails only if
all attempts fail. `CriterionResult` records total attempts and final outcome.

**Scope deliberately limited** (from debate):
- `max_attempts` only — no backoff, no `only_on_exit_codes`, no per-attempt output in results
- No flakiness index or cross-run tracking (v0.5 material)
- `[gate]` config default so project-wide policy is configurable without per-criterion noise

**Philosophical constraint**: Retry must not erode gate signal trustworthiness. The feature is
justified for commands with *known* non-determinism (network calls, port binding races in
integration tests) where single-attempt failures are not user-fixable. Documentation should
make this tradeoff explicit — retry is a last resort, not a default.

**Not included (deferred to v0.5)**:
- Exponential/fixed backoff configuration
- `only_on_exit_codes` filtering
- Per-attempt output in `CriterionResult` (schema complexity)
- Flakiness index from history aggregation

---

## Hard Deferred — v0.5.0

### Feature 7: Real-Time Gate Output Streaming
*Hard no for v0.4.0*

**Why deferred**: MCP uses stdio transport (one-to-one, request-response). SSE requires a second
HTTP transport (TcpListener, CORS, authentication). JSON-RPC notifications are possible in
principle but `rmcp` support is unclear. The primary beneficiary (TUI) doesn't consume MCP.
The sync gate evaluator wrapped in `spawn_blocking` makes channel-based emission complex.

**When to revisit**: When gate evaluation becomes async, when the TUI adopts MCP as its
transport, or when the multi-tenant orchestration use case (Smelt monitoring multiple concurrent
worktrees) makes real-time progress essential. That's v0.5 territory.

---

## Summary Table

| # | Feature | Tier | Estimate | Key Risk |
|---|---------|------|----------|----------|
| 1 | `spec_validate` | v0.4.0 Core | 3 days | PATH check env mismatch (mitigated: opt-in) |
| 2 | Context Engine crate | v0.4.0 Core | 1 week | Smelt API stability (mitigated: workspace crate) |
| 3 | `WorkSession` persistence | v0.4.0 Core | 2 weeks | Startup recovery desync |
| 4 | `gate_evaluate` capstone | v0.4.0 Core | 2.5 weeks | Prompt engineering calibration (80%→99%) |
| 5 | Gate DAG with `skip_reason` | v0.4.1 (opportunistic) | 4 days | Cycle detection required |
| 6 | Retry `max_attempts` | v0.4.1 | 4 days | Signal erosion if misused |
| 7 | Streaming output | v0.5.0 | — | Wrong milestone, wrong transport model |

**v0.4.0 Core total**: ~4.5 weeks (features 1-4 with parallelism: context engine + WorkSession overlap)

---

## Critical Implementation Order

```
Week 1:     spec_validate (solo, no deps)
            + start context engine crate (parallel)
            + start WorkSession persistence (parallel)

Week 2:     context engine crate complete
            WorkSession persistence continuing (recovery logic)

Week 3:     WorkSession complete + gate_evaluate starts
            [opportunistic: Gate DAG if evaluator refactor ahead of schedule]

Week 4-5:   gate_evaluate prompt engineering + lenient parsing + integration tests
```

---

## Key Architectural Decisions to Preserve

1. **Subprocess model**: `gate_evaluate` spawns Claude Code with `--print --output-format json`
   and parses structured output. The subprocess never calls MCP tools directly.

2. **Session management is Rust calls, not MCP round-trips**: `gate_evaluate` calls session
   persistence as direct function calls. MCP `session_*` tools are for external consumers only.

3. **`EvaluatorOutput` schema first**: Define the JSON schema before prompt engineering. Anchor
   the prompt to the schema. Use lenient `serde_json::Value` intermediate parse.

4. **`skip_reason` on `CriterionResult`, not `GateKind::Skipped`**: Preserves counting semantics.
   `result: None` remains the skip sentinel for enforcement summaries.

5. **Context engine stays in workspace**: Do not extract to separate repo until Smelt is actually
   consuming it with a stable API.
