# High-Value Feature Proposals for Assay v0.4.0

*Explorer: highvalue-ideas | Date: 2026-03-10*

---

## Feature 1: `gate_evaluate` — Diff-Aware Headless Evaluation Capstone

**Name**: Diff-Aware Headless Gate Evaluator

**What**: A single MCP tool `gate_evaluate(spec_name, base_ref?)` that computes a `git diff` between
the current worktree state and a base ref (defaults to main branch HEAD), spawns a headless Claude
Code agent in `--print --output-format json` mode with the diff as context, and collects a structured
pass/fail evaluation. The agent runs with `--allowedTools gate_report,gate_finalize` to produce a
clean structured result. The tool auto-finalizes the session and returns `run_id + pass/fail summary`
in a single round-trip.

**Why**: This is the v0.4.0 capstone and the most novel tool in the ecosystem — no MCP tool currently
does diff-aware AI evaluation. Today an agent must manually: (1) compute the diff, (2) call `gate_run`,
(3) iteratively call `gate_report` per criterion, (4) call `gate_finalize`. `gate_evaluate` collapses
this to one call. It's also the key unlock for Smelt's orchestration layer: Smelt can call
`gate_evaluate` on each worktree after feature work completes, collect structured JSON results, and
merge across sessions without any manual ceremony.

**Scope**: ~2 weeks. Depends on headless launcher (subprocess with `--print` mode), session
persistence (`WorkSession`), and structured JSON output parsing.

**Risks**:
- Headless agent may not respect tool restrictions reliably — needs validation of `--allowedTools`
  filtering.
- Diff computation in subprocesses is synchronous; large monorepos could produce huge diffs that
  overwhelm token budget before evaluation.
- The evaluator agent itself can timeout (30-min session ceiling is a hard limit).

**Dependencies**:
- Claude Code headless launcher (`--print` mode subprocess spawner) must exist.
- `WorkSession` type for linking worktrees to gate runs.
- Token-budgeted diff truncation (or integration with context engine).

---

## Feature 2: `WorkSession` — Session Record Persistence

**Name**: Work Session Lifecycle Tracking

**What**: A new `WorkSession` type persisted as JSON under `.assay/sessions/<session-id>.json`.
A `WorkSession` links: a worktree path, a spec name, an agent invocation record, and one or more
`GateRunRecord` references (via `run_id`). It tracks phase transitions:
`created → agent_running → gate_evaluated → completed | abandoned`.

Phase events are timestamped. Sessions can be listed, resumed (by re-launching the headless agent),
and queried by spec. The MCP server gains `session_create`, `session_get`, `session_list`, and
`session_update` tools.

**Why**: Without `WorkSession`, Assay has no durable link between "who did the work" and "what the
gate says about it." The history system stores gate results but not the broader agent context
(which worktree, which agent invocation, what diff it saw). `WorkSession` makes the workflow
auditable and unlocks Smelt's orchestration: Smelt needs to enumerate all sessions to decide which
are complete, which failed evaluation, and which need retry.

**Scope**: ~1 week. Primarily new types in `assay-types` and persistence logic in `assay-core`.
MCP tools are thin wrappers.

**Risks**:
- Session state machine can desync if the agent crashes mid-phase (need atomic writes + recovery
  path on load).
- Session files grow unboundedly — need pruning policy mirroring `max_history` in gate results.
- The link to gate runs (via `run_id`) is a soft reference; if history is pruned, the run record
  is gone but the session still references it.

**Dependencies**: Existing `GateRunRecord` and `history` module. No external deps.

---

## Feature 3: Spec Validation & Linting MCP Tool

**Name**: `spec_validate` — Spec Health Checker

**What**: A new MCP tool `spec_validate(spec_name?)` that statically validates specs without running
them. Checks include:
- TOML parse errors with source-location context
- Criterion name uniqueness
- Command existence on `$PATH` (via `which` / `command -v`)
- Timeout values within reasonable bounds
- AgentReport criteria have non-empty description strings (required for evaluator agent)
- Directory-based spec structure completeness (feature_spec.md present when declared)
- Cross-reference: if a spec declares `depends = [...]`, referenced specs exist

Returns structured `ValidationResult` with per-criterion diagnostics. When called without
`spec_name`, validates all specs and returns aggregated results.

**Why**: Currently, spec errors only surface at `gate_run` time — wasted agent invocations.
Agents routinely waste tokens by running gates only to hit a parse error. A pre-run validation
step lets agents (and CI) catch issues before any subprocess is spawned. It also enables a
`just validate` command for local development and a watch-mode that re-validates on spec changes.

**Scope**: ~3 days. Mostly leverages existing spec loading logic; validation rules are additive.

**Risks**:
- PATH-based command checking at validation time may differ from runtime environment (different
  shell, different user, Docker container differences).
- Cross-spec dependency validation could be expensive if the graph is large — need cycle detection.

**Dependencies**: Existing `spec_get`, TOML parsing, and spec directory discovery.

---

## Feature 4: Streaming Gate Evidence via SSE/Long-Polling

**Name**: Real-Time Gate Output Streaming

**What**: Extend `gate_run` (or add `gate_run_stream`) to emit real-time output events as command
criteria execute. Events are emitted as Server-Sent Events (SSE) from an optional HTTP endpoint
exposed by the MCP server, or alternatively via a progress-callback mechanism over the MCP
notification channel (JSON-RPC notifications). Each event contains: `criterion_name`, `stream`
(stdout|stderr), `chunk` (raw bytes up to 4 KiB), `elapsed_ms`.

The TUI's gate panel subscribes to this stream and updates in real-time. The MCP server also
exposes a `gate_output_tail(run_id)` tool that streams buffered output for in-progress runs.

**Why**: Currently gate runs are opaque until completion. For long-running commands (e.g., a
test suite taking 5+ minutes), the agent has no signal whether the command is making progress or
hung. Real-time streaming lets the TUI show live test output, enables early abort if output
signals catastrophic failure, and dramatically improves the debugging experience when a gate
unexpectedly fails.

**Scope**: ~2 weeks. Requires async gate runner (non-trivial given current sync model) or a
dedicated output-forwarding thread per criterion.

**Risks**:
- MCP protocol doesn't natively support streaming tool results (it's request-response); SSE
  requires a separate HTTP transport or creative use of JSON-RPC notifications.
- Backpressure: fast-writing commands can overwhelm the event channel.
- The sync→async boundary in gate evaluation (`spawn_blocking`) makes event emission complex.

**Dependencies**: Understanding of MCP notification protocol. Likely requires `rmcp` notification
support or a parallel HTTP endpoint. Significant complexity.

---

## Feature 5: Criterion Dependency Graph with Short-Circuit

**Name**: Gate DAG — Criteria Dependency Chains

**What**: Allow spec criteria to declare `depends_on = ["criterion-a", "criterion-b"]`. Assay builds
a dependency DAG and evaluates criteria in topological order, skipping downstream criteria when an
upstream `Required` criterion fails. The gate result includes a `skipped_because` field linking
downstream skips to their upstream failure cause.

Example: a spec has criteria `build`, `lint`, `test`, `coverage`. With `test depends_on = ["build"]`
and `coverage depends_on = ["test"]`, if `build` fails, `test` and `coverage` are auto-skipped
rather than run (saving time + tokens). The gate report shows which criteria were "blocked by
upstream failure."

**Why**: Most real-world test suites have natural ordering: you don't run tests if the build fails.
Currently Assay runs all criteria regardless of prior failures, wasting time (and potentially
causing confusing cascading errors). Dependency-aware execution reduces wall-clock time for failing
gates, reduces noise in reports, and makes the failure signal clearer (one root cause rather than
N cascading errors).

**Scope**: ~1 week. Topological sort is well-understood; the main work is (1) adding the field to
spec types, (2) building the DAG in the gate evaluator, (3) plumbing `skipped_because` through the
result types.

**Risks**:
- Circular dependency detection is required; naive cycles would deadlock or recurse.
- Parallel criterion evaluation (potential future feature) must respect DAG edges.
- User-facing error for invalid `depends_on` refs (non-existent criterion name) needs care.

**Dependencies**: Changes to `assay-types` (spec criterion type), `assay-core` gate evaluator,
and result serialization.

---

## Feature 6: Context Engine Integration — Token-Budgeted Context Windowing

**Name**: External Context Engine Crate

**What**: Extract and formalize the token estimation + context windowing logic into a standalone
Rust crate (`assay-context` or `context-engine`) that both Assay and Smelt consume. The crate
provides:
- `ContextWindow::from_budget(tokens: u32) -> ContextWindow` — builds optimal context selection
  given a token budget
- `ContextWindow::add_source(label, content, priority)` — registers content blocks with priority
- `ContextWindow::build() -> String` — resolves priority-ordered content that fits the budget
- `TokenEstimator` trait with a `tiktoken`-based impl (Cl100k encoding) and a byte-heuristic
  fallback
- Integration with Assay's diff truncation for `gate_evaluate`

**Why**: Currently Assay has ad-hoc token estimation (`estimate_tokens` MCP tool, byte-heuristic
in pruning) with no principled budget allocation. As `gate_evaluate` needs to feed a diff plus
spec content plus system prompt to the evaluator agent, it must truncate intelligently to avoid
blowing the headless agent's context. Without a principled context engine, truncation is arbitrary
and spec-specific. Extracting this as a shared crate enables Smelt to use the same windowing logic
when merging multi-session outputs.

**Scope**: ~2 weeks for initial crate + integration. Heavier if `tiktoken` bindings prove complex
(WASM-based tokenizer vs native Rust `tiktoken` crate options).

**Risks**:
- `tiktoken` Rust bindings are not officially maintained by OpenAI; third-party crates (e.g.,
  `tiktoken-rs`) may have accuracy or maintenance issues.
- Token count for Claude models differs from GPT models — may need Claude-specific approximation.
- Splitting into a separate crate adds workspace overhead and versioning coordination with Smelt.

**Dependencies**: Workspace crate split. Optional `tiktoken-rs` or byte-heuristic fallback. Must
align with Smelt's consumption API.

---

## Feature 7: Gate Retry Strategies — Flaky Test Awareness

**Name**: Criterion-Level Retry with Flakiness Tracking

**What**: Add `retry` configuration to individual gate criteria and spec-level `[gate]` defaults:
```toml
[[criteria]]
name = "integration-tests"
command = "cargo test --test integration"
retry.max_attempts = 3
retry.backoff = "exponential"        # or "fixed"
retry.backoff_initial_ms = 500
retry.only_on_exit_codes = [1]       # don't retry on 124 (timeout)
```

The gate evaluator attempts the command up to `max_attempts` times, recording each attempt's
exit code, duration, and truncated output. A criterion is considered failed only if all attempts
fail. The `CriterionResult` includes a `attempts` array for full auditability.

Additionally, Assay tracks flakiness: a `flakiness_index` computed from the ratio of "passed on
retry" to "passed on first attempt" across the last N history runs. The `gate_history` tool
surfaces this as a per-criterion annotation.

**Why**: Flaky tests are the #1 destroyer of gate signal quality. A deterministic system that marks
flaky tests as failures creates alert fatigue and causes agents to waste evaluation cycles on
spurious failures. Retry strategies make the gate system robust to known-flaky commands (network
calls, race conditions in tests). The flakiness index creates pressure to fix flaky tests by making
their systemic nature visible in history.

**Scope**: ~1 week for retry logic. ~3 additional days for flakiness tracking + history integration.

**Risks**:
- Retry loops can multiply wall-clock time significantly (3x worst case).
- Flakiness index calculation requires enough history runs to be meaningful (cold-start problem).
- `only_on_exit_codes` filtering requires careful thought: what counts as "retryable"?
- Retry state must be included in gate evidence for `gate_evaluate` diff context.

**Dependencies**: `assay-types` criterion type extension, gate evaluator changes, history
aggregation query.
