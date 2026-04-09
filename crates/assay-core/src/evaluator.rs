//! Evaluator subprocess management.
//!
//! Spawns a headless Claude Code subprocess with `--json-schema` for structured
//! per-criterion evaluation results. The parent process owns all parsing,
//! validation, and persistence — the evaluator subprocess never calls MCP tools.
//!
//! # Architecture
//!
//! ```text
//! build_evaluator_prompt() → stdin ─→ [claude -p --json-schema ...] ─→ stdout
//!                                                                        │
//!                                              parse_evaluator_output() ←┘
//!                                                        │
//!                                        map_evaluator_output() → GateRunRecord
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use chrono::Utc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use assay_types::{
    Criterion, CriterionOutcome, CriterionResult, Enforcement, EnforcementSummary,
    EvaluatorCriterionResult, EvaluatorOutput, EvaluatorRole, GateKind, GateResult, GateRunRecord,
    GateRunSummary,
};

use crate::error::EvaluatorError;
use crate::history;

/// Configuration for the evaluator subprocess.
#[derive(Debug, Clone)]
pub struct EvaluatorConfig {
    /// Claude model to use (e.g., "sonnet", "opus").
    pub model: String,
    /// Maximum time to wait for the subprocess.
    pub timeout: Duration,
    /// Number of retries for transient failures (crash/timeout).
    pub retries: u32,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            model: "sonnet".to_string(),
            timeout: Duration::from_secs(120),
            retries: 1,
        }
    }
}

/// Result of a successful evaluator invocation.
#[derive(Debug, Clone)]
pub struct EvaluatorResult {
    /// The parsed evaluator output.
    pub output: EvaluatorOutput,
    /// Wall-clock duration of the subprocess.
    pub duration: Duration,
    /// Warnings from the parse phase (e.g., unexpected envelope fields).
    pub warnings: Vec<String>,
}

/// Cached JSON Schema string for `EvaluatorOutput`.
///
/// Generated once at first access; subsequent calls return the same allocation.
static EVALUATOR_SCHEMA: LazyLock<String> = LazyLock::new(|| {
    let schema = schemars::schema_for!(EvaluatorOutput);
    serde_json::to_string(&schema).expect("schema serialization cannot fail")
});

/// Return the JSON Schema string for `EvaluatorOutput`.
///
/// Used as the `--json-schema` argument when spawning the evaluator subprocess.
/// The schema is generated once and cached for the lifetime of the process.
pub fn evaluator_schema_json() -> &'static str {
    &EVALUATOR_SCHEMA
}

/// Build the system prompt for the evaluator subprocess.
///
/// Instructs the evaluator on its role and output expectations. The `--json-schema`
/// flag handles structural enforcement, so this prompt focuses on behavior.
pub fn build_system_prompt() -> String {
    "You are a code quality evaluator. Evaluate the provided criteria against the code changes.\n\
     For each criterion, determine: pass, fail, skip (if you cannot assess), or warn (soft concern).\n\
     Provide concrete evidence and clear reasoning for each judgment.\n\
     Be precise and factual. Reference specific code when possible."
        .to_string()
}

/// Build the user prompt for the evaluator subprocess.
///
/// Constructs a structured prompt with spec description, criteria listing,
/// git diff, and additional context from the agent.
pub fn build_evaluator_prompt(
    spec_name: &str,
    spec_description: &str,
    criteria: &[Criterion],
    diff: Option<&str>,
    agent_prompt: Option<&str>,
) -> String {
    let mut sections = Vec::new();

    // Spec section
    sections.push(format!(
        "## Spec: {spec_name}\n\n{desc}",
        desc = if spec_description.is_empty() {
            "(no description provided)"
        } else {
            spec_description
        }
    ));

    // Criteria section
    let mut criteria_text = String::from("## Criteria to Evaluate\n");
    for (i, criterion) in criteria.iter().enumerate() {
        criteria_text.push_str(&format!(
            "\n### {}. {}\n\n{}\n",
            i + 1,
            criterion.name,
            criterion.description
        ));
        if let Some(prompt) = &criterion.prompt {
            criteria_text.push_str(&format!("\n**Evaluation guidance:** {prompt}\n"));
        }
    }
    sections.push(criteria_text);

    // Diff section
    let diff_text = match diff {
        Some(d) if !d.is_empty() => format!("## Code Changes (git diff)\n\n```diff\n{d}\n```"),
        _ => "## Code Changes (git diff)\n\nNo changes detected.".to_string(),
    };
    sections.push(diff_text);

    // Additional context
    if let Some(prompt) = agent_prompt
        && !prompt.is_empty()
    {
        sections.push(format!("## Additional Context\n\n{prompt}"));
    }

    // Final instruction
    sections.push("Evaluate each criterion listed above and provide your assessment.".to_string());

    sections.join("\n\n")
}

/// Known fields in the Claude Code JSON output envelope.
const KNOWN_ENVELOPE_FIELDS: &[&str] = &[
    "result",
    "structured_output",
    "session_id",
    "usage",
    "cost_usd",
    "model",
    "is_error",
];

/// Parse raw subprocess stdout into a typed `EvaluatorOutput`.
///
/// Two-phase lenient parse:
/// 1. Parse as `serde_json::Value` — check `is_error`, warn on unknown fields
/// 2. Extract `structured_output` and deserialize into `EvaluatorOutput`
///
/// Returns the parsed output and any warnings (e.g., unexpected envelope fields).
pub fn parse_evaluator_output(
    stdout: &str,
) -> Result<(EvaluatorOutput, Vec<String>), EvaluatorError> {
    let mut warnings = Vec::new();

    let envelope: serde_json::Value =
        serde_json::from_str(stdout).map_err(|e| EvaluatorError::ParseError {
            raw_output: stdout.to_string(),
            error: format!("invalid JSON: {e}"),
        })?;

    // Check for is_error flag
    if envelope
        .get("is_error")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let result = envelope
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(EvaluatorError::Crash {
            exit_code: None,
            stderr: result.to_string(),
        });
    }

    // Warn on unexpected envelope fields
    if let Some(obj) = envelope.as_object() {
        for key in obj.keys() {
            if !KNOWN_ENVELOPE_FIELDS.contains(&key.as_str()) {
                warnings.push(format!("unexpected envelope field: {key}"));
            }
        }
    }

    // Extract structured_output
    let structured =
        envelope
            .get("structured_output")
            .ok_or_else(|| EvaluatorError::NoStructuredOutput {
                raw_output: stdout.to_string(),
            })?;

    let output: EvaluatorOutput =
        serde_json::from_value(structured.clone()).map_err(|e| EvaluatorError::ParseError {
            raw_output: stdout.to_string(),
            error: format!("structured_output parse: {e}"),
        })?;

    Ok((output, warnings))
}

/// Map an `EvaluatorOutput` to a `GateRunRecord`.
///
/// Converts the four-state `CriterionOutcome` to the existing `GateResult`/`CriterionResult`
/// types, builds enforcement summaries, and generates a run ID.
///
/// Outcome mapping:
/// - `Pass` → `GateResult { passed: true }`
/// - `Fail` → `GateResult { passed: false }`
/// - `Skip` → `result: None` (criterion skipped)
/// - `Warn` → `GateResult { passed: true }` + warning collected
pub fn map_evaluator_output(
    spec_name: &str,
    output: &EvaluatorOutput,
    enforcement_map: &HashMap<String, Enforcement>,
    duration: Duration,
) -> (GateRunRecord, Vec<String>) {
    let timestamp = Utc::now();

    // Accumulator for a single criterion result.
    struct Acc {
        results: Vec<CriterionResult>,
        passed: usize,
        failed: usize,
        skipped: usize,
        enforcement_summary: EnforcementSummary,
        warnings: Vec<String>,
    }

    let Acc {
        results,
        passed,
        failed,
        skipped,
        enforcement_summary,
        mut warnings,
    } = output.criteria.iter().fold(
        Acc {
            results: Vec::new(),
            passed: 0,
            failed: 0,
            skipped: 0,
            enforcement_summary: EnforcementSummary::default(),
            warnings: Vec::new(),
        },
        |mut acc, criterion_result| {
            let enforcement = enforcement_map
                .get(&criterion_result.name)
                .copied()
                .unwrap_or(Enforcement::Required);

            match criterion_result.outcome {
                CriterionOutcome::Pass => {
                    acc.passed += 1;
                    update_enforcement_summary(&mut acc.enforcement_summary, enforcement, true);
                    acc.results.push(build_criterion_result(
                        criterion_result,
                        enforcement,
                        true,
                        timestamp,
                    ));
                }
                CriterionOutcome::Fail => {
                    acc.failed += 1;
                    update_enforcement_summary(&mut acc.enforcement_summary, enforcement, false);
                    acc.results.push(build_criterion_result(
                        criterion_result,
                        enforcement,
                        false,
                        timestamp,
                    ));
                }
                CriterionOutcome::Skip => {
                    acc.skipped += 1;
                    acc.results.push(CriterionResult {
                        criterion_name: criterion_result.name.clone(),
                        result: None,
                        enforcement,
                    });
                }
                CriterionOutcome::Warn => {
                    acc.passed += 1;
                    update_enforcement_summary(&mut acc.enforcement_summary, enforcement, true);
                    acc.warnings.push(format!(
                        "criterion '{}': {}",
                        criterion_result.name, criterion_result.reasoning
                    ));
                    acc.results.push(build_criterion_result(
                        criterion_result,
                        enforcement,
                        true,
                        timestamp,
                    ));
                }
            }
            acc
        },
    );

    // Cross-check: warn if the evaluator's self-reported verdict diverges
    // from the enforcement-derived result (required_failed > 0 means blocked).
    let enforced_blocked = enforcement_summary.required_failed > 0;
    if output.summary.passed && enforced_blocked {
        warnings.push(
            "evaluator reported overall pass, but required criteria failed — gate blocked"
                .to_string(),
        );
    } else if !output.summary.passed && !enforced_blocked {
        warnings.push(
            "evaluator reported overall fail, but no required criteria failed — gate passed"
                .to_string(),
        );
    }

    let run_id = history::generate_run_id(&timestamp);

    let record = GateRunRecord {
        run_id,
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        working_dir: None,
        summary: GateRunSummary {
            spec_name: spec_name.to_string(),
            results,
            passed,
            failed,
            skipped,
            total_duration_ms: duration.as_millis() as u64,
            enforcement: enforcement_summary,
        },
        diff_truncation: None,
    };

    (record, warnings)
}

/// Build a `CriterionResult` with a `GateResult` from an evaluator criterion result.
fn build_criterion_result(
    eval_result: &EvaluatorCriterionResult,
    enforcement: Enforcement,
    passed: bool,
    timestamp: chrono::DateTime<Utc>,
) -> CriterionResult {
    CriterionResult {
        criterion_name: eval_result.name.clone(),
        result: Some(GateResult {
            passed,
            kind: GateKind::AgentReport,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 0,
            timestamp,
            truncated: false,
            original_bytes: None,
            evidence: eval_result.evidence.clone(),
            reasoning: Some(eval_result.reasoning.clone()),
            confidence: None,
            evaluator_role: Some(EvaluatorRole::Independent),
        }),
        enforcement,
    }
}

/// Update enforcement summary counters.
fn update_enforcement_summary(
    summary: &mut EnforcementSummary,
    enforcement: Enforcement,
    passed: bool,
) {
    match (enforcement, passed) {
        (Enforcement::Required, true) => summary.required_passed += 1,
        (Enforcement::Required, false) => summary.required_failed += 1,
        (Enforcement::Advisory, true) => summary.advisory_passed += 1,
        (Enforcement::Advisory, false) => summary.advisory_failed += 1,
    }
}

/// Spawn the Claude Code evaluator subprocess, parse output, and return typed results.
///
/// Retries on transient failures (crash/timeout) up to `config.retries` times.
/// The prompt is piped via stdin to avoid command-line length limits.
pub async fn run_evaluator(
    prompt: &str,
    system_prompt: &str,
    schema_json: &str,
    config: &EvaluatorConfig,
    working_dir: &Path,
) -> Result<EvaluatorResult, EvaluatorError> {
    let max_attempts = 1 + config.retries;

    for attempt in 0..max_attempts {
        let start = std::time::Instant::now();

        match spawn_and_collect(prompt, system_prompt, schema_json, config, working_dir).await {
            Ok((stdout, duration)) => {
                let (output, warnings) = parse_evaluator_output(&stdout)?;
                return Ok(EvaluatorResult {
                    output,
                    duration,
                    warnings,
                });
            }
            Err(e) => {
                let is_retryable = matches!(
                    e,
                    EvaluatorError::Crash { .. } | EvaluatorError::Timeout { .. }
                );
                let elapsed = start.elapsed();
                tracing::warn!(
                    attempt = attempt + 1,
                    max_attempts,
                    error = %e,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "evaluator subprocess failed"
                );

                // Non-retryable errors or last attempt always return immediately.
                // Retryable errors on non-final attempts continue the loop.
                if !is_retryable || attempt + 1 >= max_attempts {
                    return Err(e);
                }
            }
        }
    }

    // Every iteration either returns Ok, returns Err, or continues (with retries
    // remaining). When all attempts are exhausted, the last iteration returns Err
    // via the guard above — this branch is never reached.
    unreachable!("run_evaluator exhausted retries without returning")
}

/// Spawn the subprocess, write prompt to stdin, collect output with timeout.
async fn spawn_and_collect(
    prompt: &str,
    system_prompt: &str,
    schema_json: &str,
    config: &EvaluatorConfig,
    working_dir: &Path,
) -> Result<(String, Duration), EvaluatorError> {
    let start = std::time::Instant::now();

    let mut child = Command::new("claude")
        .args([
            "-p",
            "--output-format",
            "json",
            "--json-schema",
            schema_json,
            "--system-prompt",
            system_prompt,
            "--tools",
            "",
            "--max-turns",
            "1",
            "--model",
            &config.model,
            "--no-session-persistence",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .current_dir(working_dir)
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EvaluatorError::NotInstalled
            } else {
                EvaluatorError::Crash {
                    exit_code: None,
                    stderr: format!("failed to spawn claude: {e}"),
                }
            }
        })?;

    // Write prompt to stdin, then close it (P2: must close before awaiting)
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| EvaluatorError::Crash {
                exit_code: None,
                stderr: format!("stdin write: {e}"),
            })?;
        // stdin dropped here, sending EOF
    }

    // Collect stdout/stderr via separate tasks, then wait on the child.
    // We use child.wait() (not wait_with_output) so we retain ownership
    // of child for kill-on-timeout.
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    let stdout_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        if let Some(mut out) = stdout_handle {
            let _ = out.read_to_end(&mut buf).await;
        }
        buf
    });
    let stderr_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        if let Some(mut err) = stderr_handle {
            let _ = err.read_to_end(&mut buf).await;
        }
        buf
    });

    // Await child exit with timeout
    let status = match tokio::time::timeout(config.timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            return Err(EvaluatorError::Crash {
                exit_code: None,
                stderr: format!("process error: {e}"),
            });
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(EvaluatorError::Timeout {
                timeout_secs: config.timeout.as_secs(),
            });
        }
    };

    let duration = start.elapsed();

    let stdout_bytes = stdout_task.await.unwrap_or_else(|e| {
        tracing::error!("stdout reader task failed: {e}");
        Vec::new()
    });
    let stderr_bytes = stderr_task.await.unwrap_or_else(|e| {
        tracing::error!("stderr reader task failed: {e}");
        Vec::new()
    });

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
        return Err(EvaluatorError::Crash {
            exit_code: status.code(),
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    Ok((stdout, duration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::criterion::When;

    // ── Schema generation ──────────────────────────────────────────

    #[test]
    fn schema_json_is_valid_and_nonempty() {
        let json = evaluator_schema_json();
        assert!(!json.is_empty());
        let value: serde_json::Value =
            serde_json::from_str(json).expect("schema should be valid JSON");
        assert!(value.is_object());
    }

    // ── Prompt construction ────────────────────────────────────────

    #[test]
    fn prompt_includes_spec_name_and_description() {
        let prompt = build_evaluator_prompt("auth-flow", "Authentication module", &[], None, None);
        assert!(prompt.contains("auth-flow"), "should include spec name");
        assert!(
            prompt.contains("Authentication module"),
            "should include description"
        );
    }

    #[test]
    fn prompt_includes_criteria_with_numbering() {
        let criteria = vec![
            Criterion {
                name: "tests-pass".to_string(),
                description: "All unit tests pass".to_string(),
                cmd: None,
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: Some("Run cargo test".to_string()),
                requirements: vec![],
                when: When::default(),
            },
            Criterion {
                name: "lint-clean".to_string(),
                description: "No clippy warnings".to_string(),
                cmd: None,
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
                when: When::default(),
            },
        ];

        let prompt = build_evaluator_prompt("spec", "desc", &criteria, None, None);
        assert!(prompt.contains("### 1. tests-pass"));
        assert!(prompt.contains("### 2. lint-clean"));
        assert!(prompt.contains("All unit tests pass"));
        assert!(prompt.contains("Run cargo test"));
    }

    #[test]
    fn prompt_includes_diff_when_provided() {
        let prompt = build_evaluator_prompt(
            "spec",
            "desc",
            &[],
            Some("+added line\n-removed line"),
            None,
        );
        assert!(prompt.contains("+added line"));
        assert!(prompt.contains("-removed line"));
        assert!(prompt.contains("```diff"));
    }

    #[test]
    fn prompt_shows_no_changes_when_diff_absent() {
        let prompt = build_evaluator_prompt("spec", "desc", &[], None, None);
        assert!(prompt.contains("No changes detected"));
    }

    #[test]
    fn prompt_includes_agent_context_when_provided() {
        let prompt =
            build_evaluator_prompt("spec", "desc", &[], None, Some("Focus on error handling"));
        assert!(prompt.contains("Focus on error handling"));
        assert!(prompt.contains("Additional Context"));
    }

    #[test]
    fn prompt_omits_agent_context_section_when_empty() {
        let prompt = build_evaluator_prompt("spec", "desc", &[], None, Some(""));
        assert!(!prompt.contains("Additional Context"));
    }

    #[test]
    fn prompt_empty_description_shows_placeholder() {
        let prompt = build_evaluator_prompt("spec", "", &[], None, None);
        assert!(prompt.contains("(no description provided)"));
    }

    // ── System prompt ──────────────────────────────────────────────

    #[test]
    fn system_prompt_is_nonempty() {
        let sp = build_system_prompt();
        assert!(!sp.is_empty());
        assert!(sp.contains("evaluator"));
    }

    // ── parse_evaluator_output ─────────────────────────────────────

    #[test]
    fn parse_valid_output() {
        let stdout = serde_json::json!({
            "result": "text result",
            "structured_output": {
                "criteria": [
                    {
                        "name": "tests-pass",
                        "outcome": "pass",
                        "reasoning": "All tests green",
                        "evidence": "42 passed, 0 failed"
                    }
                ],
                "summary": {
                    "passed": true,
                    "rationale": "Everything looks good"
                }
            },
            "session_id": "abc123",
            "usage": {},
            "cost_usd": 0.01,
            "model": "claude-sonnet-4-20250514",
            "is_error": false
        })
        .to_string();

        let (output, warnings) = parse_evaluator_output(&stdout).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(output.criteria.len(), 1);
        assert_eq!(output.criteria[0].name, "tests-pass");
        assert_eq!(output.criteria[0].outcome, CriterionOutcome::Pass);
        assert!(output.summary.passed);
    }

    #[test]
    fn parse_warns_on_unknown_envelope_fields() {
        let stdout = serde_json::json!({
            "structured_output": {
                "criteria": [],
                "summary": { "passed": true, "rationale": "ok" }
            },
            "unexpected_field": "hello",
            "another_unknown": 42
        })
        .to_string();

        let (_, warnings) = parse_evaluator_output(&stdout).unwrap();
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("unexpected_field")));
        assert!(warnings.iter().any(|w| w.contains("another_unknown")));
    }

    #[test]
    fn parse_returns_crash_on_is_error() {
        let stdout = serde_json::json!({
            "is_error": true,
            "result": "rate limit exceeded"
        })
        .to_string();

        let err = parse_evaluator_output(&stdout).unwrap_err();
        match err {
            EvaluatorError::Crash { stderr, .. } => {
                assert!(stderr.contains("rate limit exceeded"));
            }
            other => panic!("expected Crash, got: {other:?}"),
        }
    }

    #[test]
    fn parse_returns_crash_on_is_error_without_result() {
        let stdout = serde_json::json!({
            "is_error": true
        })
        .to_string();

        let err = parse_evaluator_output(&stdout).unwrap_err();
        match err {
            EvaluatorError::Crash { stderr, .. } => {
                assert_eq!(stderr, "unknown error");
            }
            other => panic!("expected Crash, got: {other:?}"),
        }
    }

    #[test]
    fn parse_returns_parse_error_on_null_structured_output() {
        let stdout = serde_json::json!({
            "structured_output": null
        })
        .to_string();

        // null is present but not deserializable to EvaluatorOutput
        let err = parse_evaluator_output(&stdout).unwrap_err();
        assert!(matches!(err, EvaluatorError::ParseError { .. }));
    }

    #[test]
    fn parse_returns_no_structured_output() {
        let stdout = serde_json::json!({
            "result": "some text",
            "is_error": false
        })
        .to_string();

        let err = parse_evaluator_output(&stdout).unwrap_err();
        assert!(matches!(err, EvaluatorError::NoStructuredOutput { .. }));
    }

    #[test]
    fn parse_returns_parse_error_on_invalid_json() {
        let err = parse_evaluator_output("not json at all").unwrap_err();
        assert!(matches!(err, EvaluatorError::ParseError { .. }));
    }

    #[test]
    fn parse_returns_parse_error_on_invalid_structured_output() {
        let stdout = serde_json::json!({
            "structured_output": {
                "wrong_field": true
            }
        })
        .to_string();

        let err = parse_evaluator_output(&stdout).unwrap_err();
        match err {
            EvaluatorError::ParseError { error, .. } => {
                assert!(error.contains("structured_output parse"));
            }
            other => panic!("expected ParseError, got: {other:?}"),
        }
    }

    #[test]
    fn parse_handles_all_four_outcomes() {
        let stdout = serde_json::json!({
            "structured_output": {
                "criteria": [
                    { "name": "a", "outcome": "pass", "reasoning": "ok" },
                    { "name": "b", "outcome": "fail", "reasoning": "bad" },
                    { "name": "c", "outcome": "skip", "reasoning": "dunno" },
                    { "name": "d", "outcome": "warn", "reasoning": "hmm" }
                ],
                "summary": { "passed": false, "rationale": "b failed" }
            }
        })
        .to_string();

        let (output, _) = parse_evaluator_output(&stdout).unwrap();
        assert_eq!(output.criteria.len(), 4);
        assert_eq!(output.criteria[0].outcome, CriterionOutcome::Pass);
        assert_eq!(output.criteria[1].outcome, CriterionOutcome::Fail);
        assert_eq!(output.criteria[2].outcome, CriterionOutcome::Skip);
        assert_eq!(output.criteria[3].outcome, CriterionOutcome::Warn);
    }

    // ── map_evaluator_output ───────────────────────────────────────

    fn make_evaluator_output() -> EvaluatorOutput {
        EvaluatorOutput {
            criteria: vec![
                EvaluatorCriterionResult {
                    name: "tests-pass".to_string(),
                    outcome: CriterionOutcome::Pass,
                    reasoning: "All tests green".to_string(),
                    evidence: Some("42 passed".to_string()),
                },
                EvaluatorCriterionResult {
                    name: "lint-clean".to_string(),
                    outcome: CriterionOutcome::Fail,
                    reasoning: "3 clippy warnings".to_string(),
                    evidence: None,
                },
                EvaluatorCriterionResult {
                    name: "docs-coverage".to_string(),
                    outcome: CriterionOutcome::Skip,
                    reasoning: "Cannot assess docs".to_string(),
                    evidence: None,
                },
                EvaluatorCriterionResult {
                    name: "naming-conventions".to_string(),
                    outcome: CriterionOutcome::Warn,
                    reasoning: "Minor naming concern".to_string(),
                    evidence: Some("foo_bar is unconventional".to_string()),
                },
            ],
            summary: assay_types::EvaluatorSummary {
                passed: false,
                rationale: "lint-clean failed".to_string(),
            },
        }
    }

    #[test]
    fn map_produces_correct_counts() {
        let output = make_evaluator_output();
        let enforcement_map = HashMap::from([
            ("tests-pass".to_string(), Enforcement::Required),
            ("lint-clean".to_string(), Enforcement::Required),
            ("docs-coverage".to_string(), Enforcement::Advisory),
            ("naming-conventions".to_string(), Enforcement::Advisory),
        ]);

        let (record, warnings) = map_evaluator_output(
            "test-spec",
            &output,
            &enforcement_map,
            Duration::from_millis(5000),
        );

        assert_eq!(record.summary.passed, 2, "pass + warn = 2 passed");
        assert_eq!(record.summary.failed, 1, "fail = 1 failed");
        assert_eq!(record.summary.skipped, 1, "skip = 1 skipped");
        assert_eq!(record.summary.results.len(), 4);
        assert_eq!(record.summary.total_duration_ms, 5000);
        assert_eq!(record.summary.spec_name, "test-spec");

        // Warn produces a warning
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("naming-conventions"));
    }

    #[test]
    fn map_enforcement_summary_correct() {
        let output = make_evaluator_output();
        let enforcement_map = HashMap::from([
            ("tests-pass".to_string(), Enforcement::Required),
            ("lint-clean".to_string(), Enforcement::Required),
            ("docs-coverage".to_string(), Enforcement::Advisory),
            ("naming-conventions".to_string(), Enforcement::Advisory),
        ]);

        let (record, _) = map_evaluator_output("spec", &output, &enforcement_map, Duration::ZERO);
        let es = &record.summary.enforcement;

        assert_eq!(es.required_passed, 1, "tests-pass is required+passed");
        assert_eq!(es.required_failed, 1, "lint-clean is required+failed");
        assert_eq!(
            es.advisory_passed, 1,
            "naming-conventions is advisory+passed (warn)"
        );
        assert_eq!(es.advisory_failed, 0, "no advisory failures");
    }

    #[test]
    fn map_defaults_enforcement_to_required() {
        let output = EvaluatorOutput {
            criteria: vec![EvaluatorCriterionResult {
                name: "unknown-criterion".to_string(),
                outcome: CriterionOutcome::Fail,
                reasoning: "bad".to_string(),
                evidence: None,
            }],
            summary: assay_types::EvaluatorSummary {
                passed: false,
                rationale: "failed".to_string(),
            },
        };

        // Empty enforcement map — should default to Required
        let (record, _) = map_evaluator_output("spec", &output, &HashMap::new(), Duration::ZERO);
        assert_eq!(record.summary.enforcement.required_failed, 1);
    }

    #[test]
    fn map_skip_produces_none_result() {
        let output = EvaluatorOutput {
            criteria: vec![EvaluatorCriterionResult {
                name: "skipped".to_string(),
                outcome: CriterionOutcome::Skip,
                reasoning: "cannot assess".to_string(),
                evidence: None,
            }],
            summary: assay_types::EvaluatorSummary {
                passed: true,
                rationale: "ok".to_string(),
            },
        };

        let (record, _) = map_evaluator_output("spec", &output, &HashMap::new(), Duration::ZERO);
        assert!(record.summary.results[0].result.is_none());
    }

    #[test]
    fn map_pass_has_agent_report_kind_and_independent_role() {
        let output = EvaluatorOutput {
            criteria: vec![EvaluatorCriterionResult {
                name: "check".to_string(),
                outcome: CriterionOutcome::Pass,
                reasoning: "looks good".to_string(),
                evidence: Some("found it".to_string()),
            }],
            summary: assay_types::EvaluatorSummary {
                passed: true,
                rationale: "ok".to_string(),
            },
        };

        let (record, _) = map_evaluator_output("spec", &output, &HashMap::new(), Duration::ZERO);
        let result = record.summary.results[0].result.as_ref().unwrap();
        assert!(result.passed);
        assert_eq!(result.kind, GateKind::AgentReport);
        assert_eq!(result.evaluator_role, Some(EvaluatorRole::Independent));
        assert_eq!(result.evidence.as_deref(), Some("found it"));
        assert_eq!(result.reasoning.as_deref(), Some("looks good"));
    }

    #[test]
    fn map_record_has_valid_run_id_and_version() {
        let output = EvaluatorOutput {
            criteria: vec![],
            summary: assay_types::EvaluatorSummary {
                passed: true,
                rationale: "empty".to_string(),
            },
        };

        let (record, _) = map_evaluator_output("spec", &output, &HashMap::new(), Duration::ZERO);
        assert!(!record.run_id.is_empty());
        assert!(!record.assay_version.is_empty());
    }

    // ── EvaluatorError display ─────────────────────────────────────

    #[test]
    fn evaluator_error_timeout_display() {
        let err = EvaluatorError::Timeout { timeout_secs: 120 };
        assert_eq!(err.to_string(), "evaluator timed out after 120s");
    }

    #[test]
    fn evaluator_error_not_installed_display() {
        let err = EvaluatorError::NotInstalled;
        assert!(err.to_string().contains("not found in PATH"));
    }

    #[test]
    fn evaluator_error_crash_display() {
        let err = EvaluatorError::Crash {
            exit_code: Some(1),
            stderr: "out of memory".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("exit code"));
        assert!(display.contains("out of memory"));
    }

    #[test]
    fn assay_error_evaluator_wrapping() {
        let err = crate::AssayError::Evaluator {
            source: EvaluatorError::NotInstalled,
        };
        let display = err.to_string();
        assert!(display.contains("gate evaluation failed"));
    }

    // ── Additional tests ────────────────────────────────────────────

    /// build-evaluator-prompt-empty-diff-untested:
    /// `diff: Some("")` should produce the same output as `diff: None`.
    #[test]
    fn prompt_empty_string_diff_matches_none_diff() {
        let with_none = build_evaluator_prompt("spec", "desc", &[], None, None);
        let with_empty = build_evaluator_prompt("spec", "desc", &[], Some(""), None);
        assert_eq!(
            with_none, with_empty,
            "Some(\"\") and None should produce identical prompts"
        );
        assert!(with_none.contains("No changes detected"));
    }

    /// map-evaluator-output-empty-criteria-no-count-assertions:
    /// Empty criteria list should yield zero counts across all fields.
    #[test]
    fn map_empty_criteria_yields_zero_counts() {
        let output = EvaluatorOutput {
            criteria: vec![],
            summary: assay_types::EvaluatorSummary {
                passed: true,
                rationale: "no criteria".to_string(),
            },
        };

        let (record, warnings) =
            map_evaluator_output("spec", &output, &HashMap::new(), Duration::ZERO);
        assert_eq!(record.summary.passed, 0);
        assert_eq!(record.summary.failed, 0);
        assert_eq!(record.summary.skipped, 0);
        assert_eq!(record.summary.results.len(), 0);
        assert!(warnings.is_empty(), "no criteria — no warnings expected");
    }

    /// map-evaluator-output-warn-required-untested:
    /// Warn outcome on a Required criterion counts as passed in enforcement.
    #[test]
    fn map_warn_on_required_criterion_counts_as_advisory_passed_not_failed() {
        let output = EvaluatorOutput {
            criteria: vec![EvaluatorCriterionResult {
                name: "strict-check".to_string(),
                outcome: CriterionOutcome::Warn,
                reasoning: "soft concern".to_string(),
                evidence: None,
            }],
            summary: assay_types::EvaluatorSummary {
                passed: true,
                rationale: "warn does not block".to_string(),
            },
        };
        let enforcement_map = HashMap::from([("strict-check".to_string(), Enforcement::Required)]);

        let (record, warnings) =
            map_evaluator_output("spec", &output, &enforcement_map, Duration::ZERO);

        // Warn is treated as passed — gate should not be blocked.
        assert_eq!(record.summary.passed, 1);
        assert_eq!(record.summary.failed, 0);
        assert_eq!(record.summary.enforcement.required_failed, 0);
        assert_eq!(record.summary.enforcement.required_passed, 1);

        // A warning message should be emitted for the warn outcome.
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("strict-check"));
    }

    /// map-pass-outcome-kind-role-test-incomplete:
    /// Fail outcome should also produce AgentReport kind and Independent role.
    #[test]
    fn map_fail_has_agent_report_kind_and_independent_role() {
        let output = EvaluatorOutput {
            criteria: vec![EvaluatorCriterionResult {
                name: "failing-check".to_string(),
                outcome: CriterionOutcome::Fail,
                reasoning: "did not pass".to_string(),
                evidence: Some("line 42 is wrong".to_string()),
            }],
            summary: assay_types::EvaluatorSummary {
                passed: false,
                rationale: "failed".to_string(),
            },
        };

        let (record, _) = map_evaluator_output("spec", &output, &HashMap::new(), Duration::ZERO);
        let result = record.summary.results[0].result.as_ref().unwrap();
        assert!(!result.passed);
        assert_eq!(result.kind, GateKind::AgentReport);
        assert_eq!(result.evaluator_role, Some(EvaluatorRole::Independent));
        assert_eq!(result.evidence.as_deref(), Some("line 42 is wrong"));
        assert_eq!(result.reasoning.as_deref(), Some("did not pass"));
    }

    /// schema-generation-test-key-structure-not-asserted:
    /// Schema JSON should contain "criteria" and "summary" as top-level properties.
    #[test]
    fn schema_json_contains_expected_top_level_properties() {
        let json = evaluator_schema_json();
        let value: serde_json::Value =
            serde_json::from_str(json).expect("schema should be valid JSON");

        // The schema should reference or define properties for EvaluatorOutput fields.
        let schema_str = json;
        assert!(
            schema_str.contains("criteria"),
            "schema should reference 'criteria' field"
        );
        assert!(
            schema_str.contains("summary"),
            "schema should reference 'summary' field"
        );
        // Verify the root is a JSON Schema object with required structural keys.
        assert!(
            value.get("properties").is_some() || value.get("$defs").is_some(),
            "schema should have 'properties' or '$defs' key: {value}"
        );
    }
}
