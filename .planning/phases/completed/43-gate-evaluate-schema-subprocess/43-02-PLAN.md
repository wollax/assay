---
phase: 43-gate-evaluate-schema-subprocess
plan: 02
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/assay-mcp/src/server.rs
autonomous: true
must_haves:
  truths:
    - gate_evaluate MCP tool accepts name, optional session_id, optional timeout, optional model parameters
    - gate_evaluate loads spec, resolves working_dir (from session if session_id provided, else from config), computes git diff, spawns evaluator subprocess, parses output, persists GateRunRecord via history::save
    - When session_id is provided, gate_evaluate loads WorkSession, uses worktree_path for diff, transitions session to GateEvaluated, appends gate_run_id — all via direct Rust calls (never MCP round-trips)
    - Evaluator subprocess is invoked with --tools "" and --max-turns 1 (ORCH-02 satisfied)
    - Lenient parse uses serde_json::Value intermediate (ORCH-03 satisfied)
    - Response includes per-criterion results with pass/fail/skip/warn status and evaluator reasoning
    - Response includes warnings field (Phase 35 pattern)
    - gate_evaluate handles subprocess timeout, crash, and missing claude binary with clear error messages
    - Tool count in server module doc comment updated to eighteen
  artifacts:
    - crates/assay-mcp/src/server.rs (gate_evaluate handler added)
  key_links:
    - gate_evaluate calls assay_core::evaluator::run_evaluator for subprocess management
    - gate_evaluate calls assay_core::evaluator::build_evaluator_prompt for prompt construction
    - gate_evaluate calls assay_core::evaluator::map_evaluator_output for result mapping
    - gate_evaluate calls assay_core::work_session functions for session auto-linking
    - gate_evaluate calls assay_core::history::save for GateRunRecord persistence
---

<objective>
Wire the gate_evaluate MCP tool handler in assay-mcp, orchestrating the full 10-step flow: load config/spec, resolve working dir, compute diff, build prompt, spawn evaluator, parse output, map results, persist record, update session.

Purpose: This is the capstone tool for v0.4.0 — a single MCP call that replaces the multi-step gate_run/gate_report/gate_finalize flow for agent-evaluated criteria. It satisfies ORCH-01, ORCH-02, and ORCH-03.

Output: gate_evaluate MCP tool handler in server.rs, tool registered in the router.
</objective>

<execution_context>
<!-- Executor agent has built-in instructions for plan execution and summary creation -->
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/pending/43-gate-evaluate-schema-subprocess/43-CONTEXT.md
@.planning/phases/pending/43-gate-evaluate-schema-subprocess/43-RESEARCH.md
@.planning/phases/pending/43-gate-evaluate-schema-subprocess/43-01-SUMMARY.md

@crates/assay-mcp/src/server.rs
@crates/assay-core/src/evaluator.rs
@crates/assay-core/src/work_session.rs
@crates/assay-core/src/gate/mod.rs
@crates/assay-core/src/history/mod.rs
@crates/assay-types/src/evaluator.rs
@crates/assay-types/src/lib.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: gate_evaluate parameter struct and MCP handler</name>
  <files>
    crates/assay-mcp/src/server.rs
  </files>
  <action>
Add the `gate_evaluate` MCP tool to `server.rs`. Follow the existing patterns established by `gate_run`, `gate_report`, and `gate_finalize`.

**Parameter struct:**
```rust
/// Parameters for the `gate_evaluate` tool.
#[derive(Deserialize, JsonSchema)]
pub struct GateEvaluateParams {
    /// The spec whose agent criteria to evaluate.
    #[schemars(description = "Spec name to evaluate (filename without .toml extension, e.g. 'auth-flow'). \
        Evaluates all criteria using a headless Claude Code subprocess as an independent evaluator. \
        Replaces the gate_run/gate_report/gate_finalize flow for agent-evaluated criteria.")]
    pub name: String,

    /// Optional work session ID to auto-link results.
    #[schemars(description = "Work session ID (from session_create). When provided, the session's \
        worktree_path is used for diff computation, and the session is transitioned to gate_evaluated \
        with the gate run ID appended.")]
    #[serde(default)]
    pub session_id: Option<String>,

    /// Override evaluator timeout in seconds.
    #[schemars(description = "Evaluator subprocess timeout in seconds (overrides config, default: 120s). \
        This is separate from gate command timeouts — LLM inference has different latency characteristics.")]
    #[serde(default)]
    pub timeout: Option<u64>,

    /// Override evaluator model.
    #[schemars(description = "Model for the evaluator subprocess (overrides config, default: 'sonnet'). \
        Accepts model aliases like 'sonnet', 'opus' or full model names.")]
    #[serde(default)]
    pub model: Option<String>,
}
```

**Handler implementation** — the 10-step flow:

1. **Load config and spec.** Use the existing `load_config` and `load_spec_entry_mcp` helpers (or their equivalents in server.rs). Extract agent criteria from the spec.

2. **Resolve working directory.** If `session_id` is provided, load the WorkSession via `assay_core::work_session::load_session` and use its `worktree_path`. Otherwise, fall back to the gate config's `working_dir` or the project root. Store the resolved path for later use.

3. **Compute git diff.** Run `git diff HEAD` in the resolved working directory (use `std::process::Command` like the existing `gate_run` handler does for diff capture). Apply `assay_core::gate::truncate_diff` with `DIFF_BUDGET_BYTES` to prevent prompt explosion. If diff capture fails, proceed with `None` diff (the evaluator can still assess structural criteria).

4. **Build evaluator prompt.** Call `assay_core::evaluator::build_evaluator_prompt` with spec name, description, criteria, diff, and agent_prompt (from the spec's criteria).

5. **Build system prompt and schema.** Call `assay_core::evaluator::build_system_prompt()` and `assay_core::evaluator::evaluator_schema_json()`.

6. **Construct EvaluatorConfig.** Use parameter overrides > config values > defaults:
   - model: `params.model` > `gates_config.evaluator_model` > "sonnet"
   - timeout: `params.timeout` (as Duration) > `gates_config.evaluator_timeout` (as Duration) > 120s
   - retries: `gates_config.evaluator_retries` > 1

7. **Spawn evaluator subprocess.** Call `assay_core::evaluator::run_evaluator(prompt, system_prompt, schema_json, config, working_dir).await`. Handle EvaluatorError:
   - `NotInstalled`: return domain error "Claude Code CLI not found..."
   - `Timeout`: return domain error "Evaluator timed out after {N}s"
   - `Crash`: return domain error with exit code and stderr
   - `ParseError`/`NoStructuredOutput`: return domain error with raw output excerpt (first 500 chars)

8. **Map to GateRunRecord.** Call `assay_core::evaluator::map_evaluator_output` with the spec name, evaluator output, enforcement map (built from spec criteria using `assay_core::gate::resolve_enforcement`), and duration. Merge any warnings from the evaluator result into the response warnings.

9. **Persist via history::save.** Call `assay_core::history::save(assay_dir, &record, max_history)`. If save fails, add to warnings but still return the result (follow the Phase 35 convention).

10. **Session auto-linking.** If `session_id` was provided:
    - Call `assay_core::work_session::record_gate_result(assay_dir, session_id, run_id, "gate_evaluate", notes)`.
    - If transition fails (e.g., session already in a terminal phase), add warning — do not fail the entire operation.

**Response format:** Return a JSON response with:
- `run_id` — the gate run record ID
- `spec_name` — the evaluated spec
- `summary` — pass/fail counts, enforcement summary
- `results` — per-criterion results with name, outcome, reasoning, evidence
- `overall_passed` — whether the gate passed overall (from evaluator summary)
- `evaluator_model` — which model was used
- `duration_ms` — total evaluation time
- `warnings` — accumulated warnings (Phase 35 pattern)
- `session_id` — if session was linked, echo it back

**Register the tool** in the `tool_router!` macro and the `list_tools` implementation. Update the module-level doc comment to say "eighteen tools" and add `gate_evaluate` to the tool list in the doc.

IMPORTANT:
- The handler must be async (evaluator is async subprocess).
- Use `tokio::task::spawn_blocking` ONLY for synchronous functions (config load, spec load, history save, session management). The evaluator subprocess call itself is already async.
- Follow the existing error handling pattern: domain errors return `CallToolResult` with `isError: true`, not `McpError`.
- The `diff` computation must happen in the resolved working directory, not the project root.
  </action>
  <verify>
`just build` compiles. `just lint` passes. The gate_evaluate tool appears in the tool router. Manual verification: confirm the tool shows up in `list_tools` output by checking the tool_router macro expansion includes gate_evaluate.
  </verify>
  <done>
gate_evaluate MCP tool is registered and handles the full 10-step flow. Compiles and passes lint. Tool description documents the subprocess model and available parameters.
  </done>
</task>

<task type="auto">
  <name>Task 2: Integration verification and final checks</name>
  <files>
    crates/assay-mcp/src/server.rs
  </files>
  <action>
Run `just ready` (fmt-check + lint + test + deny) to verify the complete phase works end-to-end.

Fix any issues that arise. Common issues to watch for:
- Missing imports in server.rs (EvaluatorOutput types, work_session functions, evaluator module)
- Clippy warnings about unused variables, needless borrows, or missing docs
- The GatesConfig deny_unknown_fields interaction with new fields
- Any schema snapshot tests that may need updating (check if the project has schema snapshot tests)

Verify that existing tests still pass — the GatesConfig changes must not break existing config parsing (the `#[serde(default)]` on new fields ensures this).

If any tests fail, diagnose the root cause before changing code. Do NOT weaken tests to make them pass.
  </action>
  <verify>
`just ready` passes (fmt-check + lint + test + deny). No regressions in existing tests.
  </verify>
  <done>
Full `just ready` passes. Phase 43 is complete: gate_evaluate MCP tool computes diff, spawns headless Claude Code evaluator, parses per-criterion results, persists GateRunRecord, and optionally auto-links work sessions.
  </done>
</task>

</tasks>

<verification>
```bash
just ready    # Full check suite: fmt-check + lint + test + deny
```

Verify ORCH-01: gate_evaluate tool exists and orchestrates the full evaluation flow.
Verify ORCH-02: Evaluator subprocess uses `--tools ""` and `--max-turns 1`.
Verify ORCH-03: Parse uses `serde_json::Value` intermediate with `structured_output` extraction.
</verification>

<success_criteria>
1. `gate_evaluate` MCP tool computes diff, spawns headless Claude Code evaluator (`--print --output-format json`), parses per-criterion results, and persists GateRunRecord (ORCH-01)
2. Evaluator subprocess never calls MCP tools — parent process owns all parsing and persistence (ORCH-02)
3. `EvaluatorOutput` JSON schema is defined before prompt engineering — lenient `serde_json::Value` intermediate parse handles unexpected fields gracefully (ORCH-03)
4. Per-criterion results include pass/fail/skip/warn status and evaluator reasoning
5. Session auto-linking works when session_id is provided
6. `just ready` passes with no regressions
</success_criteria>

<output>
After completion, create `.planning/phases/43-gate-evaluate-schema-subprocess/43-02-SUMMARY.md`
</output>
