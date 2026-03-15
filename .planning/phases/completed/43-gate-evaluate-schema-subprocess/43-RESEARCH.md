# Phase 43: gate_evaluate Schema & Subprocess - Research

**Researched:** 2026-03-15
**Confidence:** HIGH (codebase patterns well-established, Claude Code CLI docs verified)

## Summary

Phase 43 adds a new `gate_evaluate` MCP tool that replaces the existing multi-step gate_run/gate_report/gate_finalize flow for agent-evaluated criteria with a single call. It spawns a headless Claude Code subprocess (`claude -p --output-format json --json-schema '...'`), parses the structured JSON output, and persists a `GateRunRecord`. The parent process owns all parsing, validation, and persistence — the evaluator subprocess never calls MCP tools.

Key research findings:
1. Claude Code CLI natively supports `--json-schema` for validated structured output — use this instead of prompt-instructed JSON
2. The existing `gate::session` module provides the finalization/persistence pattern to reuse
3. `tokio::process::Command` is the right async subprocess primitive (already in tokio's `full` feature)
4. `serde_json::Value` intermediate parse is the correct lenient parsing strategy for the `EvaluatorOutput` schema

## Standard Stack

### Claude Code CLI Invocation (HIGH confidence)
**Use:** `claude -p --output-format json --json-schema '<schema>' --system-prompt '<prompt>' --tools "" --max-turns 1`

Key flags:
- `--print` / `-p`: Non-interactive mode, prints response and exits
- `--output-format json`: Returns structured JSON with `result`, `session_id`, `usage`, `cost_usd` fields. When combined with `--json-schema`, the validated output appears in the `structured_output` field
- `--json-schema '<json_schema>'`: Validates output against a JSON Schema definition. The evaluator's response is extracted from `response.structured_output`
- `--system-prompt '<text>'`: Replaces the entire system prompt (we want full control, not Claude Code's default behavior)
- `--tools ""`: Disables all built-in tools — the evaluator should reason, not execute
- `--max-turns 1`: Prevents the evaluator from entering agentic loops (single reasoning turn)
- `--model '<model>'`: Configurable model selection
- `--no-session-persistence`: Avoids writing evaluator sessions to disk

The prompt is piped via stdin to avoid command-line length limits:
```
echo "<prompt_text>" | claude -p --output-format json --json-schema '...' --system-prompt '...' --tools "" --max-turns 1 --model '<model>' --no-session-persistence
```

### Async Subprocess Management (HIGH confidence)
**Use:** `tokio::process::Command` from the `tokio` crate (already a workspace dependency with `full` features).

Pattern:
```rust
use tokio::process::Command;
use tokio::io::AsyncWriteExt;

let mut child = Command::new("claude")
    .args(["-p", "--output-format", "json", "--json-schema", &schema_json, ...])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .current_dir(&working_dir)
    .spawn()?;

// Write prompt to stdin
if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(prompt.as_bytes()).await?;
    // stdin is dropped here, sending EOF
}

// Wait with timeout
let output = tokio::time::timeout(timeout, child.wait_with_output()).await;
```

### Lenient JSON Parsing (HIGH confidence)
**Use:** Two-phase parse with `serde_json::Value` then `serde_json::from_value::<EvaluatorOutput>()`.

The Claude Code `--json-schema` flag handles schema validation at the LLM level, but the parent process still needs lenient parsing because:
1. The outer JSON envelope (`result`, `structured_output`, etc.) may evolve
2. Extra fields should warn, not fail
3. Partial/malformed output from crashes needs graceful handling

Pattern:
```rust
// Phase 1: Parse raw stdout as generic JSON
let raw: serde_json::Value = serde_json::from_str(&stdout)?;

// Phase 2: Extract structured_output field
let structured = raw.get("structured_output")
    .ok_or_else(|| /* missing structured_output error */)?;

// Phase 3: Warn on unexpected top-level fields
for key in raw.as_object().map(|m| m.keys()).into_iter().flatten() {
    if !KNOWN_FIELDS.contains(key.as_str()) {
        tracing::warn!("unexpected field in evaluator output: {key}");
    }
}

// Phase 4: Deserialize into typed struct (with #[serde(default)] for resilience)
let output: EvaluatorOutput = serde_json::from_value(structured.clone())?;
```

### Existing Crate Dependencies (HIGH confidence)
No new crate dependencies needed. Everything required is already in the workspace:
- `tokio` (full features, includes `tokio::process`)
- `serde_json` (Value type, from_value)
- `serde` (derive, Serialize/Deserialize)
- `schemars` (JsonSchema derive for generating the --json-schema argument)
- `chrono` (timestamps)
- `tracing` (warn logging for unexpected fields)
- `assay-types` (GateRunRecord, CriterionResult, etc.)
- `assay-core` (gate::session, history::save, config::load)

## Architecture Patterns

### Pattern 1: EvaluatorOutput Schema (in assay-types)

Define the schema the evaluator subprocess must produce. This goes in `assay-types` because the MCP server and core both need it.

```rust
/// Outcome of evaluating a single criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CriterionOutcome {
    Pass,
    Fail,
    Skip,
    Warn,
}

/// Per-criterion evaluation result from the evaluator subprocess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvaluatorCriterionResult {
    /// Criterion name (must match a criterion in the spec).
    pub name: String,
    /// Four-state outcome.
    pub outcome: CriterionOutcome,
    /// Free-text reasoning explaining the judgment.
    pub reasoning: String,
    /// Concrete evidence observed (optional but encouraged).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// Complete evaluator output: per-criterion results + overall summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvaluatorOutput {
    /// Per-criterion evaluation results.
    pub criteria: Vec<EvaluatorCriterionResult>,
    /// Overall summary with aggregate judgment.
    pub summary: EvaluatorSummary,
}

/// Aggregate summary from the evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvaluatorSummary {
    /// Whether the gate passed overall (all required criteria pass).
    pub passed: bool,
    /// Brief rationale for the overall judgment.
    pub rationale: String,
}
```

The JSON Schema for `--json-schema` is generated at runtime via `schemars::schema_for!(EvaluatorOutput)` and serialized to a JSON string.

### Pattern 2: gate_evaluate MCP Tool (in assay-mcp)

New tool following the existing MCP handler pattern. The flow:

1. Load config and spec (reuse `load_config`, `load_spec_entry_mcp`)
2. Resolve working directory — if `session_id` provided, load `WorkSession` and use its `worktree_path`
3. Compute git diff (`git diff HEAD` in working_dir)
4. Build evaluator prompt (diff + criteria + spec description + agent_prompt)
5. Generate JSON Schema string from `EvaluatorOutput`
6. Spawn Claude Code subprocess with timeout
7. Parse output: `serde_json::Value` -> extract `structured_output` -> `EvaluatorOutput`
8. Map `EvaluatorOutput` -> `CriterionResult` vec -> `GateRunSummary` -> `GateRunRecord`
9. Persist via `history::save`
10. Optionally update `WorkSession` phase if `session_id` provided

```rust
#[derive(Deserialize, JsonSchema)]
pub struct GateEvaluateParams {
    pub name: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub model: Option<String>,
}
```

### Pattern 3: Subprocess Execution Module (in assay-core)

New module `assay_core::gate::evaluate` (or `assay_core::evaluator`) encapsulating:

```rust
pub struct EvaluatorConfig {
    pub model: String,
    pub timeout: Duration,
    pub retries: u32,
}

pub enum EvaluatorError {
    Timeout,
    Crash { exit_code: Option<i32>, stderr: String },
    ParseError { raw_output: String, error: String },
    NoStructuredOutput { raw_output: String },
}

pub struct EvaluatorResult {
    pub output: EvaluatorOutput,
    pub duration: Duration,
    pub warnings: Vec<String>,
    pub raw_output: String, // For debugging
}

/// Spawn evaluator subprocess, parse output, return typed result.
pub async fn run_evaluator(
    prompt: &str,
    schema_json: &str,
    system_prompt: &str,
    config: &EvaluatorConfig,
    working_dir: &Path,
) -> Result<EvaluatorResult, EvaluatorError>
```

This keeps subprocess management in `assay-core`, not in the MCP server.

### Pattern 4: Prompt Construction (in assay-core)

Build the evaluator prompt from spec context. All context is in a single prompt (single subprocess call for all criteria — cost/speed over independence).

```rust
pub fn build_evaluator_prompt(
    spec_description: &str,
    criteria: &[Criterion],
    diff: Option<&str>,
    agent_prompt: Option<&str>,
) -> String
```

The prompt structure:
```
You are evaluating whether code changes meet the acceptance criteria for a spec.

## Spec: {name}
{description}

## Criteria to Evaluate
{for each criterion: name, description, prompt (if AgentReport)}

## Code Changes (git diff)
{diff or "No changes detected"}

## Additional Context
{agent_prompt if provided}

Evaluate each criterion and provide your assessment.
```

The system prompt instructs the output format:
```
You are a code quality evaluator. Evaluate the provided criteria against the code changes.
For each criterion, determine: pass, fail, skip (if you cannot assess), or warn (soft concern).
Provide concrete evidence and clear reasoning for each judgment.
```

### Pattern 5: Config Extension (in assay-types)

Extend `GatesConfig` with evaluator settings:

```rust
pub struct GatesConfig {
    // ... existing fields ...

    /// Default model for evaluator subprocess. Default: "sonnet".
    #[serde(default = "default_evaluator_model")]
    pub evaluator_model: String,

    /// Maximum retries for transient evaluator failures. Default: 1.
    #[serde(default = "default_evaluator_retries")]
    pub evaluator_retries: u32,

    /// Evaluator subprocess timeout in seconds. Default: 120.
    #[serde(default = "default_evaluator_timeout")]
    pub evaluator_timeout: u64,
}
```

Use a separate `evaluator_timeout` key (not the gate timeout cascade) because evaluator subprocess timing is fundamentally different from command gate timing — LLM inference has different latency characteristics.

### Pattern 6: CriterionOutcome to GateResult Mapping

Map the four-state `CriterionOutcome` to existing `GateResult`:

| CriterionOutcome | GateResult.passed | Enforcement Impact |
|---|---|---|
| Pass | true | Counts toward passed |
| Fail | false | Required: gate fails. Advisory: warning |
| Skip | N/A (result = None) | Counts as skipped |
| Warn | true (with warning) | Logged as warning, does not fail gate |

`Warn` maps to `passed = true` because it should not fail the gate, but the warning is captured in the response.

### Pattern 7: Session Auto-Linking

When `session_id` is provided:
1. Load `WorkSession` from `.assay/sessions/{id}.json`
2. Use `worktree_path` from session to compute diff
3. After successful evaluation, transition session to `GateEvaluated` phase
4. Append gate run ID to `session.gate_runs`
5. Save updated session

This uses direct Rust function calls via `assay_core::work_session`, never MCP round-trips.

## Don't Hand-Roll

1. **JSON Schema generation** — Use `schemars::schema_for!(EvaluatorOutput)` to generate the `--json-schema` argument. Do not manually write JSON Schema strings.

2. **Subprocess timeout** — Use `tokio::time::timeout` wrapping `child.wait_with_output()`. Do not implement manual polling loops (the existing `gate/mod.rs` polling pattern is for `std::process`, not needed with tokio).

3. **Run ID generation** — Use existing `history::generate_run_id()` for the GateRunRecord run_id.

4. **History persistence** — Use existing `history::save()` for persisting GateRunRecord. Do not duplicate the atomic-write pattern.

5. **Session management** — Use existing `work_session::save_session()` and `work_session::transition_session()`. Do not reimplement session phase transitions.

6. **Enforcement resolution** — Use existing `gate::resolve_enforcement()` for determining criterion enforcement levels.

7. **Diff capture and truncation** — Use existing `gate::truncate_diff()` and the git diff pattern from the `gate_run` handler.

8. **Config loading** — Use existing `config::load()` and the `resolve_working_dir` helper pattern.

## Common Pitfalls

### P1: Claude Code --json-schema output location (HIGH confidence)
The structured output is NOT in the `result` field. When `--json-schema` is used, the validated JSON appears in the `structured_output` field of the response envelope. The `result` field contains the text representation. Always extract from `structured_output`.

### P2: Subprocess stdin must be closed (HIGH confidence)
When piping the prompt via stdin, the stdin handle MUST be dropped (closed) before awaiting the output. Claude Code waits for EOF on stdin before processing. Pattern: take stdin with `.take()`, write, then let it drop.

### P3: Empty/missing `claude` binary (HIGH confidence)
`claude` may not be installed or may not be in PATH. The subprocess spawn will fail with `io::Error`. Handle this gracefully with a clear error message: "Claude Code CLI (`claude`) not found in PATH. Install it from https://code.claude.com".

### P4: Evaluator output may contain thinking blocks (MEDIUM confidence)
With `--output-format json`, thinking blocks may appear in the response metadata. The `structured_output` field should be clean, but verify during testing. The lenient parse handles this.

### P5: GatesConfig deny_unknown_fields (HIGH confidence)
`GatesConfig` uses `#[serde(deny_unknown_fields)]`. Adding new fields (`evaluator_model`, `evaluator_retries`, `evaluator_timeout`) requires updating the struct — existing config files without these fields will use defaults via `#[serde(default)]`, but any config file with unknown fields from a newer version will fail to parse on an older version. This is the existing convention.

### P6: Subprocess environment inheritance (MEDIUM confidence)
The Claude Code subprocess inherits the parent's environment, including `ANTHROPIC_API_KEY`. This is correct and desired — the evaluator needs API access. No explicit env setup needed.

### P7: Retry vs. partial results (MEDIUM confidence)
On crash/timeout, record a gate-level error (no per-criterion results) as decided in CONTEXT.md. Do NOT attempt to parse partial stdout from a crashed process — it's unreliable. Retries are the recovery path for transient failures.

### P8: Large diff overflow (MEDIUM confidence)
The diff is included in the prompt. Phase 44 handles context budgeting, but Phase 43 should use the existing `truncate_diff` with `DIFF_BUDGET_BYTES` (32 KiB) to prevent prompt explosion. This is already the pattern in `gate_run`.

### P9: Model alias resolution (MEDIUM confidence)
Claude Code accepts model aliases like `"sonnet"` and `"opus"` in addition to full model names like `"claude-sonnet-4-20250514"`. The config field should accept either form and pass through directly to `--model`.

### P10: Test isolation — subprocess mocking (HIGH confidence)
Unit tests must NOT spawn real Claude Code subprocesses. The `run_evaluator` function should accept a trait or be structured so tests can inject mock output. Pattern: extract the subprocess invocation into a small function that tests can replace, or test the parsing/mapping logic separately from the subprocess spawning.

## Code Examples

### Example 1: Generating JSON Schema for --json-schema flag
```rust
fn evaluator_schema_json() -> String {
    let schema = schemars::schema_for!(EvaluatorOutput);
    serde_json::to_string(&schema).expect("schema serialization cannot fail")
}
```

### Example 2: Spawning the evaluator subprocess
```rust
async fn spawn_evaluator(
    prompt: &str,
    config: &EvaluatorConfig,
    working_dir: &Path,
) -> Result<Output, EvaluatorError> {
    let schema_json = evaluator_schema_json();

    let mut child = tokio::process::Command::new("claude")
        .args([
            "-p",
            "--output-format", "json",
            "--json-schema", &schema_json,
            "--system-prompt", &system_prompt,
            "--tools", "",
            "--max-turns", "1",
            "--model", &config.model,
            "--no-session-persistence",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(working_dir)
        .spawn()
        .map_err(|e| EvaluatorError::Crash {
            exit_code: None,
            stderr: format!("failed to spawn claude: {e}"),
        })?;

    // Write prompt to stdin, then close it
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await.map_err(|e| {
            EvaluatorError::Crash { exit_code: None, stderr: format!("stdin write: {e}") }
        })?;
    }

    // Await with timeout
    match tokio::time::timeout(config.timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(EvaluatorError::Crash {
            exit_code: None,
            stderr: format!("process error: {e}"),
        }),
        Err(_) => {
            let _ = child.kill().await;
            Err(EvaluatorError::Timeout)
        }
    }
}
```

### Example 3: Parsing evaluator output with lenient parse
```rust
const KNOWN_ENVELOPE_FIELDS: &[&str] = &[
    "result", "structured_output", "session_id", "usage", "cost_usd", "model", "is_error",
];

fn parse_evaluator_output(
    stdout: &str,
) -> Result<(EvaluatorOutput, Vec<String>), EvaluatorError> {
    let mut warnings = Vec::new();

    let envelope: serde_json::Value = serde_json::from_str(stdout)
        .map_err(|e| EvaluatorError::ParseError {
            raw_output: stdout.to_string(),
            error: format!("invalid JSON: {e}"),
        })?;

    // Warn on unexpected envelope fields
    if let Some(obj) = envelope.as_object() {
        for key in obj.keys() {
            if !KNOWN_ENVELOPE_FIELDS.contains(&key.as_str()) {
                warnings.push(format!("unexpected envelope field: {key}"));
            }
        }
    }

    // Check for is_error flag
    if envelope.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false) {
        let result = envelope.get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(EvaluatorError::Crash {
            exit_code: None,
            stderr: result.to_string(),
        });
    }

    let structured = envelope.get("structured_output")
        .ok_or_else(|| EvaluatorError::NoStructuredOutput {
            raw_output: stdout.to_string(),
        })?;

    let output: EvaluatorOutput = serde_json::from_value(structured.clone())
        .map_err(|e| EvaluatorError::ParseError {
            raw_output: stdout.to_string(),
            error: format!("structured_output parse: {e}"),
        })?;

    Ok((output, warnings))
}
```

### Example 4: Mapping EvaluatorOutput to GateRunRecord
```rust
fn map_to_gate_run_summary(
    spec_name: &str,
    output: &EvaluatorOutput,
    enforcement_map: &HashMap<String, Enforcement>,
    duration_ms: u64,
) -> GateRunSummary {
    let mut results = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut enforcement_summary = EnforcementSummary::default();

    for criterion_result in &output.criteria {
        let enforcement = enforcement_map
            .get(&criterion_result.name)
            .copied()
            .unwrap_or(Enforcement::Required);

        match criterion_result.outcome {
            CriterionOutcome::Pass => {
                passed += 1;
                // ... update enforcement_summary
                results.push(CriterionResult {
                    criterion_name: criterion_result.name.clone(),
                    result: Some(GateResult { passed: true, /* ... */ }),
                    enforcement,
                });
            }
            CriterionOutcome::Fail => {
                failed += 1;
                // ...
            }
            CriterionOutcome::Skip => {
                skipped += 1;
                results.push(CriterionResult {
                    criterion_name: criterion_result.name.clone(),
                    result: None,
                    enforcement,
                });
            }
            CriterionOutcome::Warn => {
                passed += 1; // Warn does not fail the gate
                // ...
            }
        }
    }

    GateRunSummary {
        spec_name: spec_name.to_string(),
        results,
        passed,
        failed,
        skipped,
        total_duration_ms: duration_ms,
        enforcement: enforcement_summary,
    }
}
```

## Sources

- Claude Code CLI reference: https://code.claude.com/docs/en/cli-reference
- Claude Code headless/programmatic usage: https://code.claude.com/docs/en/headless
- Codebase: `crates/assay-mcp/src/server.rs` — existing MCP tool patterns (gate_run, gate_report, gate_finalize)
- Codebase: `crates/assay-core/src/gate/mod.rs` — synchronous gate evaluation, subprocess spawning with `std::process::Command`
- Codebase: `crates/assay-core/src/gate/session.rs` — session lifecycle (create, report, finalize, build_finalized_record)
- Codebase: `crates/assay-types/src/session.rs` — AgentEvaluation, AgentSession, Confidence, EvaluatorRole
- Codebase: `crates/assay-types/src/gate_run.rs` — GateRunRecord, GateRunSummary, CriterionResult
- Codebase: `crates/assay-types/src/work_session.rs` — WorkSession, SessionPhase, PhaseTransition
- Codebase: `crates/assay-types/src/lib.rs` — Config, GatesConfig (deny_unknown_fields)

## Metadata

- **Research duration:** Single session
- **Domains covered:** Claude Code CLI flags, async subprocess management in Rust/tokio, JSON schema generation (schemars), lenient serde parsing, existing assay codebase patterns
- **Key decisions for planner:**
  1. Use `--json-schema` flag (not prompt-instructed JSON) for structured output — the CLI validates it
  2. Single subprocess for all criteria (cost/speed) — not one per criterion
  3. Separate `evaluator_timeout` config key — LLM inference has different latency than shell commands
  4. `EvaluatorOutput` schema in `assay-types` — shared between MCP server and core
  5. Subprocess execution logic in `assay-core` — MCP server is a thin wrapper
  6. `--tools ""` disables all tools — evaluator reasons only, never executes
  7. Warn outcome maps to `passed = true` — soft concerns don't fail gates
  8. System prompt via `--system-prompt` (full replacement) — complete control over evaluator behavior
  9. Prompt via stdin pipe — avoids command-line length limits for large diffs
  10. Session auto-linking: transition to `GateEvaluated` and append gate_run ID when `session_id` provided
