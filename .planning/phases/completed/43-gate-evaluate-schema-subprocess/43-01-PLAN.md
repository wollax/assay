---
phase: 43-gate-evaluate-schema-subprocess
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/assay-types/src/evaluator.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-core/src/evaluator.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/src/error.rs
autonomous: true
must_haves:
  truths:
    - EvaluatorOutput schema exists in assay-types with CriterionOutcome (Pass/Fail/Skip/Warn), EvaluatorCriterionResult, EvaluatorSummary, and EvaluatorOutput — all derive Serialize, Deserialize, JsonSchema
    - GatesConfig has evaluator_model, evaluator_retries, and evaluator_timeout fields with serde(default) and sensible defaults (sonnet, 1, 120)
    - assay-core exposes async run_evaluator() that spawns Claude Code subprocess with --json-schema, --tools "", --max-turns 1, pipes prompt via stdin, parses lenient JSON output
    - Lenient parse extracts structured_output from envelope, warns on unknown fields, checks is_error flag
    - build_evaluator_prompt constructs prompt from spec description, criteria, diff, and agent_prompt
    - map_evaluator_output converts EvaluatorOutput to GateRunRecord using CriterionOutcome-to-GateResult mapping (Pass=passed, Fail=failed, Skip=result None, Warn=passed+warning)
    - EvaluatorError enum covers Timeout, Crash, ParseError, NoStructuredOutput, and NotInstalled
    - Parse and mapping logic has unit tests independent of subprocess spawning
  artifacts:
    - crates/assay-types/src/evaluator.rs
    - crates/assay-core/src/evaluator.rs
  key_links:
    - EvaluatorOutput is used by assay-core evaluator module for --json-schema generation via schemars::schema_for!
    - GatesConfig evaluator fields feed into EvaluatorConfig construction
    - map_evaluator_output produces GateRunRecord compatible with history::save
---

<objective>
Define the EvaluatorOutput JSON schema types and build the core evaluator module — subprocess execution, prompt construction, output parsing, and result mapping.

Purpose: Establish the typed contract between the Claude Code evaluator subprocess and assay, plus all the domain logic needed to spawn, parse, and map evaluator results. This is the foundation that Plan 02's MCP tool handler builds on.

Output: New `evaluator` modules in both assay-types and assay-core, plus GatesConfig extension with evaluator settings.
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

@crates/assay-types/src/lib.rs
@crates/assay-types/src/gate.rs
@crates/assay-types/src/gate_run.rs
@crates/assay-types/src/criterion.rs
@crates/assay-types/src/enforcement.rs
@crates/assay-core/src/lib.rs
@crates/assay-core/src/error.rs
@crates/assay-core/src/gate/mod.rs
@crates/assay-core/src/gate/session.rs
@crates/assay-core/src/history/mod.rs
@crates/assay-core/src/work_session.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: EvaluatorOutput schema types and GatesConfig extension</name>
  <files>
    crates/assay-types/src/evaluator.rs
    crates/assay-types/src/lib.rs
  </files>
  <action>
Create `crates/assay-types/src/evaluator.rs` with the following types. All types must derive Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, and have doc comments (the crate has `#![deny(missing_docs)]`).

1. **`CriterionOutcome`** enum with `#[serde(rename_all = "snake_case")]`:
   - `Pass` — criterion satisfied
   - `Fail` — criterion not satisfied
   - `Skip` — evaluator could not assess
   - `Warn` — soft concern, does not fail gate

2. **`EvaluatorCriterionResult`** struct:
   - `name: String` — criterion name (must match spec)
   - `outcome: CriterionOutcome`
   - `reasoning: String` — free-text reasoning
   - `evidence: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`

3. **`EvaluatorSummary`** struct:
   - `passed: bool` — overall gate pass/fail
   - `rationale: String` — brief overall judgment

4. **`EvaluatorOutput`** struct:
   - `criteria: Vec<EvaluatorCriterionResult>`
   - `summary: EvaluatorSummary`

Register all four types with `inventory::submit!` for schema generation (follow the pattern in `gate_run.rs`).

In `crates/assay-types/src/lib.rs`:
- Add `pub mod evaluator;`
- Add re-exports: `pub use evaluator::{CriterionOutcome, EvaluatorCriterionResult, EvaluatorOutput, EvaluatorSummary};`

**GatesConfig extension** — in `crates/assay-types/src/lib.rs`, add three fields to `GatesConfig`:

```rust
/// Default model for the evaluator subprocess. Defaults to "sonnet".
#[serde(default = "default_evaluator_model")]
pub evaluator_model: String,

/// Maximum retries for transient evaluator subprocess failures. Defaults to 1.
#[serde(default = "default_evaluator_retries")]
pub evaluator_retries: u32,

/// Evaluator subprocess timeout in seconds. Defaults to 120.
#[serde(default = "default_evaluator_timeout")]
pub evaluator_timeout: u64,
```

Add the corresponding default functions:
- `fn default_evaluator_model() -> String { "sonnet".to_string() }`
- `fn default_evaluator_retries() -> u32 { 1 }`
- `fn default_evaluator_timeout() -> u64 { 120 }`

IMPORTANT: GatesConfig has `#[serde(deny_unknown_fields)]`. The new fields MUST use `#[serde(default = "...")]` so existing config files without these fields parse correctly.
  </action>
  <verify>
`just build` compiles. Schema generation for EvaluatorOutput works: add a test in evaluator.rs that calls `schemars::schema_for!(EvaluatorOutput)` and verifies it serializes to a non-empty JSON string. Add serde roundtrip tests for EvaluatorOutput and CriterionOutcome.
  </verify>
  <done>
All four evaluator types exist in assay-types with correct derives, doc comments, and schema registration. GatesConfig has three new evaluator fields with defaults. Tests pass for schema generation and serde roundtrip.
  </done>
</task>

<task type="auto">
  <name>Task 2: Core evaluator module — subprocess, prompt, parsing, mapping</name>
  <files>
    crates/assay-core/src/evaluator.rs
    crates/assay-core/src/lib.rs
    crates/assay-core/src/error.rs
  </files>
  <action>
**Error types** — add to `crates/assay-core/src/error.rs`:

Add a new `EvaluatorError` enum (NOT inside AssayError — this is a standalone error type for the evaluator module, similar to how `ConfigError` and `SpecError` are standalone). Define it with `#[derive(Debug, Error)]`:

```rust
/// Errors from the evaluator subprocess.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EvaluatorError {
    /// Evaluator subprocess timed out.
    #[error("evaluator timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    /// Evaluator subprocess crashed or exited with non-zero status.
    #[error("evaluator crashed (exit code: {exit_code:?}): {stderr}")]
    Crash { exit_code: Option<i32>, stderr: String },

    /// Evaluator output could not be parsed as JSON.
    #[error("evaluator output parse error: {error}")]
    ParseError { raw_output: String, error: String },

    /// Evaluator output missing structured_output field.
    #[error("evaluator output missing structured_output field")]
    NoStructuredOutput { raw_output: String },

    /// Claude Code CLI not found in PATH.
    #[error("Claude Code CLI (`claude`) not found in PATH. Install from https://claude.ai/code")]
    NotInstalled,
}
```

Also add a variant to `AssayError` for wrapping evaluator errors:
```rust
/// Evaluator subprocess failed.
#[error("gate evaluation failed: {source}")]
Evaluator {
    #[source]
    source: EvaluatorError,
},
```

**Core module** — create `crates/assay-core/src/evaluator.rs` with:

1. **`EvaluatorConfig`** struct:
   - `model: String`
   - `timeout: std::time::Duration`
   - `retries: u32`

2. **`EvaluatorResult`** struct:
   - `output: EvaluatorOutput`
   - `duration: std::time::Duration`
   - `warnings: Vec<String>`

3. **`build_evaluator_prompt(spec_name, spec_description, criteria, diff, agent_prompt) -> String`**
   Constructs the user prompt with sections for spec description, criteria listing (name + description + prompt for each), git diff, and additional context. Follow the prompt structure from RESEARCH.md Pattern 4. Include all criteria regardless of kind — the evaluator assesses them holistically.

4. **`build_system_prompt() -> String`**
   Returns the system prompt instructing the evaluator on its role and output expectations. Keep it concise — the `--json-schema` flag handles structural enforcement.

5. **`evaluator_schema_json() -> String`**
   Generates JSON Schema from `schemars::schema_for!(EvaluatorOutput)` and serializes to string. Cache-friendly (pure function, no side effects).

6. **`parse_evaluator_output(stdout: &str) -> Result<(EvaluatorOutput, Vec<String>), EvaluatorError>`**
   Two-phase lenient parse:
   - Parse stdout as `serde_json::Value`
   - Check `is_error` flag — if true, return `Crash` error with the `result` field as stderr
   - Warn on unexpected top-level fields (known: result, structured_output, session_id, usage, cost_usd, model, is_error)
   - Extract `structured_output` field — return `NoStructuredOutput` if missing
   - Deserialize `structured_output` into `EvaluatorOutput` — return `ParseError` on failure
   - Return `(EvaluatorOutput, warnings)`

7. **`map_evaluator_output(spec_name, output, enforcement_map, duration_ms) -> GateRunRecord`**
   Maps EvaluatorOutput to GateRunRecord:
   - For each criterion result, look up enforcement from the map (default Required)
   - Map CriterionOutcome: Pass -> GateResult{passed:true}, Fail -> GateResult{passed:false}, Skip -> result:None, Warn -> GateResult{passed:true} + add warning to a warnings collection
   - GateResult fields: kind=AgentReport, evidence from EvaluatorCriterionResult.evidence, reasoning from EvaluatorCriterionResult.reasoning, evaluator_role=Some(EvaluatorRole::Independent)
   - Build GateRunSummary with correct pass/fail/skip counts and EnforcementSummary
   - Generate run_id via `history::generate_run_id()`, set assay_version
   - NOTE: `generate_run_id` is `pub(crate)` — it's accessible since this module is inside assay-core

8. **`async fn run_evaluator(prompt, system_prompt, schema_json, config, working_dir) -> Result<EvaluatorResult, EvaluatorError>`**
   Spawns the Claude Code subprocess:
   - Build args: `-p`, `--output-format json`, `--json-schema <schema>`, `--system-prompt <system_prompt>`, `--tools ""`, `--max-turns 1`, `--model <model>`, `--no-session-persistence`
   - Spawn via `tokio::process::Command` with stdin/stdout/stderr piped, current_dir set
   - Write prompt to stdin, then drop stdin handle (P2: must close before awaiting)
   - Await with `tokio::time::timeout(config.timeout, child.wait_with_output())`
   - On timeout: kill child, return `Timeout`
   - On spawn failure with NotFound errno: return `NotInstalled` (P3)
   - On non-zero exit: return `Crash`
   - On success: call `parse_evaluator_output` on stdout, return `EvaluatorResult`
   - Implement retry logic: on `Crash` or `Timeout`, retry up to `config.retries` times

In `crates/assay-core/src/lib.rs`, add `pub mod evaluator;`.

**Testing approach** (P10): Write unit tests for `parse_evaluator_output`, `map_evaluator_output`, `build_evaluator_prompt`, and `evaluator_schema_json` independently from subprocess spawning. Do NOT spawn real Claude Code subprocesses in tests. Test parse with fabricated JSON strings that match the Claude Code output envelope format.
  </action>
  <verify>
`just build` compiles. `just test` passes with new unit tests. Tests cover: schema JSON generation is non-empty and valid JSON, prompt construction includes all sections, parse handles valid output/missing structured_output/is_error/unknown fields, map produces correct pass/fail/skip/warn counts and enforcement summary.
  </verify>
  <done>
Core evaluator module exists in assay-core with all 8 components (config, result, prompt builders, schema gen, parser, mapper, async runner). EvaluatorError added to error.rs. Unit tests pass for all parse/map/prompt logic without subprocess dependency. `just ready` passes.
  </done>
</task>

</tasks>

<verification>
```bash
just build    # All crates compile
just test     # All tests pass including new evaluator tests
just lint     # No clippy warnings
just fmt-check # Formatting correct
```
</verification>

<success_criteria>
1. `EvaluatorOutput` JSON schema is defined in assay-types with CriterionOutcome (4-state), EvaluatorCriterionResult, EvaluatorSummary — all with serde/schemars derives
2. `GatesConfig` extended with evaluator_model, evaluator_retries, evaluator_timeout (backward-compatible via serde defaults)
3. `run_evaluator` async function spawns Claude Code with correct flags, pipes prompt via stdin, parses lenient JSON, handles timeout/crash/missing-binary gracefully
4. `parse_evaluator_output` extracts structured_output from envelope with unknown-field warnings
5. `map_evaluator_output` correctly maps 4-state outcomes to GateRunRecord with enforcement summary
6. All parse/map/prompt logic tested without subprocess dependency
</success_criteria>

<output>
After completion, create `.planning/phases/43-gate-evaluate-schema-subprocess/43-01-SUMMARY.md`
</output>
