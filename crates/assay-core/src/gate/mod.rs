//! Quality gate evaluation.
//!
//! Gates are checkpoints that verify work meets quality criteria
//! before allowing progression through the workflow. This module
//! provides synchronous gate evaluation — command execution with
//! timeout enforcement, file existence checks, and aggregate result
//! summaries.
//!
//! # Async Usage
//!
//! All functions in this module are synchronous. When calling from an
//! async context (e.g., MCP server handlers), wrap calls with
//! [`tokio::task::spawn_blocking`]:
//!
//! ```ignore
//! let result = tokio::task::spawn_blocking(move || {
//!     gate::evaluate(&criterion, &working_dir, timeout)
//! }).await??;
//! ```

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use chrono::Utc;

use assay_types::{
    Criterion, CriterionKind, CriterionResult, Enforcement, EnforcementSummary, GateCriterion,
    GateKind, GateResult, GateRunSummary, GateSection, GatesSpec, Spec,
};

use crate::error::{AssayError, Result};

pub mod session;

/// Classification of command execution errors based on shell exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandErrorKind {
    /// Exit code 127: command not found in PATH.
    NotFound,
    /// Exit code 126: command found but not executable.
    NotExecutable,
}

/// Extract the binary name (first whitespace-delimited token) from a command string.
///
/// Returns an empty string if the command is empty or whitespace-only.
pub(crate) fn extract_binary(cmd: &str) -> &str {
    cmd.split_whitespace().next().unwrap_or("")
}

/// Classify a shell exit code into a known error kind.
///
/// Returns `Some(CommandErrorKind)` for exit codes 127 (not found) and 126
/// (not executable), `None` for all other codes.
pub fn classify_exit_code(code: i32) -> Option<CommandErrorKind> {
    match code {
        127 => Some(CommandErrorKind::NotFound),
        126 => Some(CommandErrorKind::NotExecutable),
        _ => None,
    }
}

/// Format a user-facing error message for command execution failures.
///
/// Extracts the binary name from the command string and produces an
/// actionable message based on the error kind.
pub fn format_command_error(cmd: &str, kind: CommandErrorKind) -> String {
    let binary = extract_binary(cmd);
    match kind {
        CommandErrorKind::NotFound => {
            format!("command '{binary}' not found. Is it installed and in PATH?")
        }
        CommandErrorKind::NotExecutable => {
            format!("command '{binary}' found but not executable. Check file permissions.")
        }
    }
}

/// Byte budget per output stream (stdout/stderr) for head+tail truncation (32 KiB).
const STREAM_BUDGET: usize = 32_768;

/// Polling interval for `try_wait` timeout loop.
const POLL_INTERVAL_MS: u64 = 50;

/// Minimum timeout floor in seconds.
const MIN_TIMEOUT_SECS: u64 = 1;

/// Evaluate a single criterion as a gate.
///
/// Derives `GateKind` from criterion fields: if `cmd` is `Some`, it's
/// `GateKind::Command`; if `path` is `Some` (and `cmd` is `None`), it's
/// `GateKind::FileExists`; if both are `None`, it's `GateKind::AlwaysPass`.
///
/// `working_dir` is required — this function never inherits the process CWD.
/// `timeout` is the maximum wall-clock time for the command to complete.
///
/// # Async Usage
///
/// This function is synchronous. From async code, use:
/// ```ignore
/// tokio::task::spawn_blocking(move || gate::evaluate(&criterion, &dir, timeout)).await??
/// ```
pub fn evaluate(
    criterion: &Criterion,
    working_dir: &Path,
    timeout: Duration,
) -> Result<GateResult> {
    // AgentReport criteria cannot be evaluated standalone — they are
    // evaluated through the session lifecycle (gate_report + gate_finalize).
    if criterion.kind == Some(CriterionKind::AgentReport) {
        return Err(AssayError::InvalidCriterion {
            spec_name: String::new(),
            criterion_name: criterion.name.clone(),
        });
    }

    match (&criterion.cmd, &criterion.path) {
        (Some(cmd), _) => evaluate_command(cmd, working_dir, timeout),
        (None, Some(path)) => evaluate_file_exists(path, working_dir),
        (None, None) => evaluate_always_pass(),
    }
}

/// Evaluate all criteria in a spec sequentially.
///
/// Skips criteria without `cmd` or `path` (records as skipped in summary). Uses
/// [`resolve_timeout`] for each criterion's timeout. Individual criterion
/// failures are captured in `GateResult`, not propagated as errors. If
/// [`evaluate`] returns an `Err` (spawn failure), it's captured as a
/// failed `CriterionResult` with error details in stderr.
///
/// # Async Usage
///
/// This function is synchronous. From async code, use:
/// ```ignore
/// tokio::task::spawn_blocking(move || {
///     gate::evaluate_all(&spec, &dir, cli_timeout, config_timeout)
/// }).await?
/// ```
pub fn evaluate_all(
    spec: &Spec,
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary {
    let criteria: Vec<(Criterion, Enforcement)> = spec
        .criteria
        .iter()
        .map(|c| {
            let enforcement = resolve_enforcement(c.enforcement, spec.gate.as_ref());
            (c.clone(), enforcement)
        })
        .collect();
    evaluate_criteria(
        &spec.name,
        criteria,
        working_dir,
        cli_timeout,
        config_timeout,
    )
}

/// Evaluate all criteria in a `GatesSpec` sequentially.
///
/// Equivalent to [`evaluate_all`] but for the directory-based `GatesSpec`
/// format. Each `GateCriterion` is converted to a `Criterion` for evaluation.
/// The `requirements` field on `GateCriterion` is not used during evaluation
/// (it's metadata for traceability).
pub fn evaluate_all_gates(
    gates: &GatesSpec,
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary {
    let criteria: Vec<(Criterion, Enforcement)> = gates
        .criteria
        .iter()
        .map(|gc| {
            let enforcement = resolve_enforcement(gc.enforcement, gates.gate.as_ref());
            (to_criterion(gc), enforcement)
        })
        .collect();
    evaluate_criteria(
        &gates.name,
        criteria,
        working_dir,
        cli_timeout,
        config_timeout,
    )
}

/// Shared evaluation loop for both legacy specs and gate specs.
///
/// Each entry pairs a `Criterion` with its resolved `Enforcement` level.
/// Handles AgentReport skipping, no-cmd-no-path skipping, timeout resolution,
/// evaluation dispatch, and result/enforcement summary accumulation.
fn evaluate_criteria(
    spec_name: &str,
    criteria: Vec<(Criterion, Enforcement)>,
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary {
    let start = Instant::now();
    let mut results = Vec::with_capacity(criteria.len());
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut enforcement_summary = EnforcementSummary::default();

    for (criterion, resolved_enforcement) in &criteria {
        // AgentReport criteria are evaluated through the session lifecycle,
        // not the synchronous evaluate path. Mark as skipped (pending).
        if criterion.kind == Some(CriterionKind::AgentReport) {
            skipped += 1;
            results.push(CriterionResult {
                criterion_name: criterion.name.clone(),
                result: None,
                enforcement: *resolved_enforcement,
            });
            continue;
        }

        if criterion.cmd.is_none() && criterion.path.is_none() {
            skipped += 1;
            results.push(CriterionResult {
                criterion_name: criterion.name.clone(),
                result: None,
                enforcement: *resolved_enforcement,
            });
            continue;
        }

        let timeout = resolve_timeout(cli_timeout, criterion.timeout, config_timeout);

        match evaluate(criterion, working_dir, timeout) {
            Ok(mut gate_result) => {
                if gate_result.passed {
                    passed += 1;
                    match resolved_enforcement {
                        Enforcement::Required => enforcement_summary.required_passed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_passed += 1,
                    }
                } else {
                    // Enrich stderr for exit code 127/126 when a command was run.
                    // Integration-tested via classify_exit_code and format_command_error unit tests.
                    if let Some(code) = gate_result.exit_code
                        && let Some(kind) = classify_exit_code(code)
                        && let Some(cmd) = criterion.cmd.as_deref()
                    {
                        let hint = format_command_error(cmd, kind);
                        if !gate_result.stderr.is_empty() {
                            gate_result.stderr.push('\n');
                        }
                        gate_result.stderr.push_str(&hint);
                    }
                    failed += 1;
                    match resolved_enforcement {
                        Enforcement::Required => enforcement_summary.required_failed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_failed += 1,
                    }
                }
                results.push(CriterionResult {
                    criterion_name: criterion.name.clone(),
                    result: Some(gate_result),
                    enforcement: *resolved_enforcement,
                });
            }
            Err(err) => {
                failed += 1;
                match resolved_enforcement {
                    Enforcement::Required => enforcement_summary.required_failed += 1,
                    Enforcement::Advisory => enforcement_summary.advisory_failed += 1,
                }
                let stderr = enriched_error_display(&err, criterion.cmd.as_deref());
                results.push(CriterionResult {
                    criterion_name: criterion.name.clone(),
                    result: Some(GateResult {
                        passed: false,
                        kind: gate_kind_for(criterion),
                        stdout: String::new(),
                        stderr,
                        exit_code: None,
                        duration_ms: 0,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: *resolved_enforcement,
                });
            }
        }
    }

    GateRunSummary {
        spec_name: spec_name.to_string(),
        results,
        passed,
        failed,
        skipped,
        total_duration_ms: start.elapsed().as_millis() as u64,
        enforcement: enforcement_summary,
    }
}

/// Format an error message for gate evaluation failures, enriching spawn
/// errors (NotFound / PermissionDenied) with actionable hints when a command
/// is available.
pub fn enriched_error_display(err: &AssayError, cmd: Option<&str>) -> String {
    if let AssayError::GateExecution { source, .. } = err
        && let Some(cmd) = cmd
    {
        let kind = match source.kind() {
            std::io::ErrorKind::NotFound => Some(CommandErrorKind::NotFound),
            std::io::ErrorKind::PermissionDenied => Some(CommandErrorKind::NotExecutable),
            _ => None,
        };
        if let Some(kind) = kind {
            return format!(
                "gate evaluation error: {err}\n{}",
                format_command_error(cmd, kind)
            );
        }
    }
    format!("gate evaluation error: {err}")
}

/// Convert a `GateCriterion` to a `Criterion` for evaluation.
///
/// Now that `GateCriterion` is a type alias for `Criterion`, this is a
/// trivial clone. Kept as a named function for call-site readability.
pub fn to_criterion(gc: &GateCriterion) -> Criterion {
    gc.clone()
}

/// Derive the `GateKind` for error reporting from a criterion's fields.
///
/// Mirrors the dispatch logic in [`evaluate`] so that error results
/// carry the correct gate kind even when evaluation fails before
/// producing a `GateResult`.
fn gate_kind_for(criterion: &Criterion) -> GateKind {
    if criterion.kind == Some(CriterionKind::AgentReport) {
        return GateKind::AgentReport;
    }
    match (&criterion.cmd, &criterion.path) {
        (Some(cmd), _) => GateKind::Command { cmd: cmd.clone() },
        (None, Some(path)) => GateKind::FileExists { path: path.clone() },
        (None, None) => GateKind::AlwaysPass,
    }
}

/// Resolve the effective enforcement level for a criterion.
///
/// Precedence: per-criterion override > spec-level `[gate]` default > Required.
pub fn resolve_enforcement(
    criterion_enforcement: Option<Enforcement>,
    gate_section: Option<&GateSection>,
) -> Enforcement {
    criterion_enforcement.unwrap_or_else(|| {
        gate_section
            .map(|g| g.enforcement)
            .unwrap_or(Enforcement::Required)
    })
}

/// Resolve the effective timeout from three tiers of configuration.
///
/// Precedence: CLI `--timeout` > per-criterion `timeout` > config global > 300s default.
/// Enforces a minimum floor of 1 second.
pub fn resolve_timeout(
    cli_timeout: Option<u64>,
    criterion_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> Duration {
    let seconds = cli_timeout
        .or(criterion_timeout)
        .or(config_timeout)
        .unwrap_or(300);
    Duration::from_secs(seconds.max(MIN_TIMEOUT_SECS))
}

/// Evaluate a file existence gate.
///
/// Resolves `path` relative to `working_dir` and checks whether the
/// file exists. No process execution, no timeout needed.
///
/// Dispatched from [`evaluate`] when a criterion has `path: Some(...)` and
/// `cmd: None`.
pub fn evaluate_file_exists(path: &str, working_dir: &Path) -> Result<GateResult> {
    let start = Instant::now();

    // Reject absolute paths and traversal outside working_dir
    let raw = Path::new(path);
    if raw.is_absolute() {
        return Ok(GateResult {
            passed: false,
            kind: GateKind::FileExists {
                path: path.to_string(),
            },
            stdout: String::new(),
            stderr: format!("path must be relative, got: {path}"),
            exit_code: None,
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: Utc::now(),
            truncated: false,
            original_bytes: None,
            evidence: None,
            reasoning: None,
            confidence: None,
            evaluator_role: None,
        });
    }

    let full_path = working_dir.join(path);
    let exists = full_path.exists();

    Ok(GateResult {
        passed: exists,
        kind: GateKind::FileExists {
            path: path.to_string(),
        },
        stdout: String::new(),
        stderr: if exists {
            String::new()
        } else {
            format!("file not found: {}", full_path.display())
        },
        exit_code: None,
        duration_ms: start.elapsed().as_millis() as u64,
        timestamp: Utc::now(),
        truncated: false,
        original_bytes: None,
        evidence: None,
        reasoning: None,
        confidence: None,
        evaluator_role: None,
    })
}

/// Execute a shell command and capture its result.
///
/// Spawns `sh -c <cmd>` with piped stdout/stderr, uses reader threads
/// for deadlock-free pipe draining, and polls `try_wait` for timeout
/// enforcement. On timeout, kills the process group (Unix) and reaps
/// the zombie.
fn evaluate_command(cmd: &str, working_dir: &Path, timeout: Duration) -> Result<GateResult> {
    let start = Instant::now();

    let mut command = Command::new("sh");
    command
        .args(["-c", cmd])
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // On Unix, put child in its own process group so timeout kills the tree
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command
        .spawn()
        .map_err(|source| AssayError::GateExecution {
            cmd: cmd.to_string(),
            working_dir: working_dir.to_path_buf(),
            source,
        })?;

    // Take pipe handles before waiting (avoids borrow issues)
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // Spawn reader threads to drain pipes (prevents deadlock).
    // Errors are captured alongside the buffer so the caller can
    // include partial output in GateResult even if a read fails.
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stdout) = stdout_handle
            && let Err(e) = std::io::Read::read_to_end(&mut stdout, &mut buf)
        {
            return (buf, Some(e));
        }
        (buf, None)
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stderr) = stderr_handle
            && let Err(e) = std::io::Read::read_to_end(&mut stderr, &mut buf)
        {
            return (buf, Some(e));
        }
        (buf, None)
    });

    // Poll for completion with timeout
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    // Kill the process group (Unix) or direct child (non-Unix)
                    #[cfg(unix)]
                    {
                        // SAFETY: child.id() returns a u32; process_group(0) set
                        // pgid == pid, so killpg sends SIGKILL to the entire group.
                        let pid = child.id() as libc::pid_t;
                        unsafe { libc::killpg(pid, libc::SIGKILL) };
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = child.kill();
                    }
                    // Reap zombie — ignore errors (process may already be gone)
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            }
            Err(e) => {
                return Err(AssayError::GateExecution {
                    cmd: cmd.to_string(),
                    working_dir: working_dir.to_path_buf(),
                    source: e,
                });
            }
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // Join reader threads (safe: process is dead, pipes will EOF).
    // Thread panics and pipe read errors are surfaced as warnings
    // appended to the corresponding output stream.
    let (stdout_bytes, stdout_read_err) = stdout_thread.join().unwrap_or_else(|_| {
        (
            Vec::new(),
            Some(std::io::Error::other("stdout reader thread panicked")),
        )
    });
    let (stderr_bytes, stderr_read_err) = stderr_thread.join().unwrap_or_else(|_| {
        (
            Vec::new(),
            Some(std::io::Error::other("stderr reader thread panicked")),
        )
    });

    let mut stdout_raw = String::from_utf8_lossy(&stdout_bytes).into_owned();
    let mut stderr_raw = String::from_utf8_lossy(&stderr_bytes).into_owned();

    // Append pipe read errors so they appear in evidence
    if let Some(e) = stdout_read_err {
        if !stdout_raw.is_empty() {
            stdout_raw.push('\n');
        }
        stdout_raw.push_str(&format!("[pipe read error: {e}]"));
    }
    if let Some(e) = stderr_read_err {
        if !stderr_raw.is_empty() {
            stderr_raw.push('\n');
        }
        stderr_raw.push_str(&format!("[pipe read error: {e}]"));
    }

    // Apply head+tail truncation independently per stream
    let stdout_result = truncate_head_tail(stdout_raw, STREAM_BUDGET);
    let stderr_result = truncate_head_tail(stderr_raw, STREAM_BUDGET);

    let truncated = stdout_result.truncated || stderr_result.truncated;
    let original_bytes = if truncated {
        Some(stdout_result.original_bytes as u64 + stderr_result.original_bytes as u64)
    } else {
        None
    };

    // Build result based on whether timeout occurred
    match status {
        Some(exit_status) => {
            let exit_code = exit_status.code();
            Ok(GateResult {
                passed: exit_status.success(),
                kind: GateKind::Command {
                    cmd: cmd.to_string(),
                },
                stdout: stdout_result.output,
                stderr: stderr_result.output,
                exit_code,
                duration_ms,
                timestamp: Utc::now(),
                truncated,
                original_bytes,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            })
        }
        None => {
            // Timeout: append timeout notice to stderr
            let timeout_stderr = if stderr_result.output.is_empty() {
                format!("[timed out after {}s]", timeout.as_secs())
            } else {
                format!(
                    "{}\n[timed out after {}s]",
                    stderr_result.output,
                    timeout.as_secs()
                )
            };

            Ok(GateResult {
                passed: false,
                kind: GateKind::Command {
                    cmd: cmd.to_string(),
                },
                stdout: stdout_result.output,
                stderr: timeout_stderr,
                exit_code: None,
                duration_ms,
                timestamp: Utc::now(),
                truncated,
                original_bytes,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            })
        }
    }
}

/// Return an immediately-passing result for AlwaysPass gates.
fn evaluate_always_pass() -> Result<GateResult> {
    Ok(GateResult {
        passed: true,
        kind: GateKind::AlwaysPass,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        duration_ms: 0,
        timestamp: Utc::now(),
        truncated: false,
        original_bytes: None,
        evidence: None,
        reasoning: None,
        confidence: None,
        evaluator_role: None,
    })
}

/// Result of applying head+tail truncation to a string.
///
/// When truncation is applied, the output contains the first portion (head)
/// and last portion (tail) of the original input, separated by a marker
/// indicating how many bytes were omitted.
#[derive(Debug)]
pub(crate) struct TruncationResult {
    /// The possibly-truncated output string.
    pub(crate) output: String,
    /// Whether truncation was applied.
    pub(crate) truncated: bool,
    /// Original byte length of the input.
    pub(crate) original_bytes: usize,
}

/// Truncate output using a head+tail strategy with a byte budget.
///
/// Accepts an owned `String` to avoid an allocation on the passthrough path
/// (when no truncation is needed, the original `String` is returned as-is).
///
/// If the input fits within `budget` bytes, it is returned unchanged.
/// Otherwise, the output is split into a head (~33%) and tail (~67%)
/// section separated by a `[truncated: X bytes omitted]` marker.
///
/// The marker itself is overhead and does not count against the budget,
/// so the total output length may be up to `budget + marker_len`.
/// UTF-8 character boundaries are respected using `floor_char_boundary`
/// and `ceil_char_boundary` to avoid splitting multi-byte sequences.
///
/// A `budget` of 0 produces a marker-only output with no content bytes.
pub(crate) fn truncate_head_tail(input: String, budget: usize) -> TruncationResult {
    let original_bytes = input.len();

    if original_bytes <= budget {
        return TruncationResult {
            output: input,
            truncated: false,
            original_bytes,
        };
    }

    let head_budget = budget / 3;
    let tail_budget = budget - head_budget;

    let head_end = input.floor_char_boundary(head_budget);
    let tail_start = input.ceil_char_boundary(original_bytes.saturating_sub(tail_budget));

    let (head, tail) = if tail_start <= head_end {
        // Overlap: fall back to tail-only
        let safe_start = input.ceil_char_boundary(original_bytes.saturating_sub(budget));
        ("", &input[safe_start..])
    } else {
        (&input[..head_end], &input[tail_start..])
    };

    debug_assert!(
        head.len() + tail.len() <= original_bytes,
        "head ({}) + tail ({}) > original_bytes ({})",
        head.len(),
        tail.len(),
        original_bytes,
    );
    let omitted = original_bytes - head.len() - tail.len();
    let output = format!("{head}\n[truncated: {omitted} bytes omitted]\n{tail}");

    TruncationResult {
        output,
        truncated: true,
        original_bytes,
    }
}

/// Extract file paths from git diff headers.
///
/// Parses `diff --git a/<path> b/<path>` headers, returning the `b/` path
/// for each file in the diff. The `b/` path represents the destination (post-change)
/// file path, which is the conventional choice for display.
pub fn extract_diff_files(diff: &str) -> Vec<String> {
    diff.lines()
        .filter(|l| l.starts_with("diff --git "))
        .filter_map(|l| l.split(" b/").nth(1).map(str::to_string))
        .collect()
}

/// Truncate a diff string using head+tail strategy.
///
/// Returns `(truncated_output, was_truncated, original_byte_length)`.
/// If input is empty, returns `(None, false, None)`.
pub fn truncate_diff(raw: &str, budget: usize) -> (Option<String>, bool, Option<usize>) {
    if raw.is_empty() {
        return (None, false, None);
    }
    let result = truncate_head_tail(raw.to_owned(), budget);
    (
        Some(result.output),
        result.truncated,
        Some(result.original_bytes),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_diff_files ─────────────────────────────────────────

    #[test]
    fn extract_diff_files_empty_diff() {
        assert_eq!(extract_diff_files(""), Vec::<String>::new());
    }

    #[test]
    fn extract_diff_files_single_file() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\nindex abc..def 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n";
        assert_eq!(extract_diff_files(diff), vec!["src/main.rs"]);
    }

    #[test]
    fn extract_diff_files_multiple_files() {
        let diff = "diff --git a/foo.rs b/foo.rs\nsome content\ndiff --git a/bar.rs b/bar.rs\nmore content\ndiff --git a/baz.rs b/baz.rs\n";
        assert_eq!(extract_diff_files(diff), vec!["foo.rs", "bar.rs", "baz.rs"]);
    }

    #[test]
    fn extract_diff_files_no_headers() {
        let diff = "@@ -1,3 +1,4 @@\n context\n-removed\n+added\n another line\n";
        assert_eq!(extract_diff_files(diff), Vec::<String>::new());
    }

    #[test]
    fn extract_diff_files_path_with_spaces() {
        let diff = "diff --git a/path with spaces/file.rs b/path with spaces/file.rs\n";
        assert_eq!(extract_diff_files(diff), vec!["path with spaces/file.rs"]);
    }

    // ── evaluate: command execution ────────────────────────────────

    #[test]
    fn evaluate_echo_hello() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "echo test".to_string(),
            description: "runs echo".to_string(),
            cmd: Some("echo hello".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        assert!(result.passed, "echo should pass");
        assert!(
            result.stdout.contains("hello"),
            "stdout should contain 'hello', got: {:?}",
            result.stdout
        );
        assert_eq!(result.exit_code, Some(0));
        assert!(
            result.duration_ms > 0 || cfg!(miri),
            "duration_ms should be non-zero"
        );
        assert!(matches!(result.kind, GateKind::Command { .. }));
    }

    #[test]
    fn evaluate_failing_command() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "fail test".to_string(),
            description: "runs failing cmd".to_string(),
            cmd: Some("sh -c 'echo fail >&2 && exit 1'".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        assert!(!result.passed, "failing command should not pass");
        assert!(
            result.stderr.contains("fail"),
            "stderr should contain 'fail', got: {:?}",
            result.stderr
        );
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    #[cfg_attr(not(unix), ignore)]
    fn evaluate_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "timeout test".to_string(),
            description: "runs slow cmd".to_string(),
            cmd: Some("sleep 10".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(1)).unwrap();

        assert!(!result.passed, "timed-out command should not pass");
        assert_eq!(result.exit_code, None, "timed-out should have no exit code");
        assert!(
            result.stderr.contains("timed out"),
            "stderr should mention timeout, got: {:?}",
            result.stderr
        );
    }

    // ── evaluate: always pass ──────────────────────────────────────

    #[test]
    fn evaluate_always_pass_criterion() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "descriptive".to_string(),
            description: "no cmd".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        assert!(result.passed);
        assert!(matches!(result.kind, GateKind::AlwaysPass));
        assert_eq!(result.duration_ms, 0);
    }

    // ── evaluate_file_exists ───────────────────────────────────────

    #[test]
    fn evaluate_file_exists_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("exists.txt"), "content").unwrap();

        let result = evaluate_file_exists("exists.txt", dir.path()).unwrap();

        assert!(result.passed);
        assert!(matches!(
            result.kind,
            GateKind::FileExists { ref path } if path == "exists.txt"
        ));
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn evaluate_file_exists_missing() {
        let dir = tempfile::tempdir().unwrap();

        let result = evaluate_file_exists("missing.txt", dir.path()).unwrap();

        assert!(!result.passed);
        assert!(
            result.stderr.contains("file not found"),
            "stderr should mention file not found, got: {:?}",
            result.stderr
        );
    }

    // ── working directory ──────────────────────────────────────────

    #[test]
    fn evaluate_working_dir_is_respected() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "pwd test".to_string(),
            description: "checks working dir".to_string(),
            cmd: Some("pwd".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        // On macOS, tempdir may use /private/var prefix while pwd resolves symlinks
        let expected = dir.path().canonicalize().unwrap();
        assert!(
            result.stdout.trim() == expected.to_str().unwrap()
                || result
                    .stdout
                    .trim()
                    .ends_with(dir.path().file_name().unwrap().to_str().unwrap()),
            "stdout should contain the working dir path, got: {:?}, expected: {:?}",
            result.stdout.trim(),
            expected
        );
    }

    // ── evaluate_command: independent stream truncation (GATE-04) ──

    #[test]
    #[cfg_attr(not(unix), ignore)]
    fn evaluate_command_independent_stream_truncation() {
        let dir = tempfile::tempdir().unwrap();

        // Generate large stdout (> STREAM_BUDGET) but small stderr
        let large_size = STREAM_BUDGET + 1024;
        let cmd = format!("printf '%0.s.' $(seq 1 {large_size}) && echo 'small stderr' >&2");
        let criterion = Criterion {
            name: "stream truncation test".to_string(),
            description: "tests independent truncation".to_string(),
            cmd: Some(cmd),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(30)).unwrap();

        assert!(result.truncated, "should be truncated");
        assert!(
            result.original_bytes.is_some(),
            "original_bytes should be set"
        );
        assert!(
            result.stdout.contains("[truncated: "),
            "stdout should have truncation marker, got len: {}",
            result.stdout.len()
        );
        assert!(
            !result.stderr.contains("[truncated: "),
            "stderr should NOT be truncated"
        );
        assert!(
            result.stderr.contains("small stderr"),
            "stderr should contain original content"
        );
    }

    // ── resolve_timeout ────────────────────────────────────────────

    #[test]
    fn resolve_timeout_cli_wins() {
        let timeout = resolve_timeout(Some(30), Some(60), Some(120));
        assert_eq!(timeout, Duration::from_secs(30));
    }

    #[test]
    fn resolve_timeout_criterion_wins_over_config() {
        let timeout = resolve_timeout(None, Some(60), Some(120));
        assert_eq!(timeout, Duration::from_secs(60));
    }

    #[test]
    fn resolve_timeout_config_used() {
        let timeout = resolve_timeout(None, None, Some(120));
        assert_eq!(timeout, Duration::from_secs(120));
    }

    #[test]
    fn resolve_timeout_default_300s() {
        let timeout = resolve_timeout(None, None, None);
        assert_eq!(timeout, Duration::from_secs(300));
    }

    #[test]
    fn resolve_timeout_minimum_floor() {
        let timeout = resolve_timeout(Some(0), None, None);
        assert_eq!(timeout, Duration::from_secs(1));
    }

    // ── evaluate_all ───────────────────────────────────────────────

    #[test]
    fn evaluate_all_mixed_criteria() {
        let dir = tempfile::tempdir().unwrap();
        let spec = Spec {
            name: "mixed".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                Criterion {
                    name: "passes".to_string(),
                    description: "will pass".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "descriptive".to_string(),
                    description: "no cmd".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "fails".to_string(),
                    description: "will fail".to_string(),
                    cmd: Some("false".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        assert_eq!(summary.spec_name, "mixed");
        assert_eq!(summary.passed, 1, "one criterion should pass");
        assert_eq!(summary.failed, 1, "one criterion should fail");
        assert_eq!(summary.skipped, 1, "one criterion should be skipped");
        assert_eq!(summary.results.len(), 3);

        // Check descriptive criterion was skipped
        assert!(summary.results[1].result.is_none());
    }

    #[test]
    fn evaluate_all_captures_spawn_failure() {
        let dir = tempfile::tempdir().unwrap();
        let spec = Spec {
            name: "bad-cmd".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![Criterion {
                name: "impossible".to_string(),
                description: "nonexistent binary".to_string(),
                cmd: Some("/nonexistent/binary/that/does/not/exist".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        assert_eq!(summary.failed, 1, "spawn failure should count as failed");
        assert_eq!(summary.passed, 0);
        let result = summary.results[0].result.as_ref().unwrap();
        assert!(!result.passed);
    }

    // ── evaluate_all_gates ────────────────────────────────────────────

    #[test]
    fn evaluate_all_gates_mixed_criteria() {
        let dir = tempfile::tempdir().unwrap();
        let gates = GatesSpec {
            name: "mixed".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                GateCriterion {
                    name: "passes".to_string(),
                    description: "will pass".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec!["REQ-FUNC-001".to_string()],
                },
                GateCriterion {
                    name: "descriptive".to_string(),
                    description: "no cmd".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                GateCriterion {
                    name: "fails".to_string(),
                    description: "will fail".to_string(),
                    cmd: Some("false".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec!["REQ-SEC-001".to_string()],
                },
            ],
        };

        let summary = evaluate_all_gates(&gates, dir.path(), None, None);

        assert_eq!(summary.spec_name, "mixed");
        assert_eq!(summary.passed, 1, "one criterion should pass");
        assert_eq!(summary.failed, 1, "one criterion should fail");
        assert_eq!(summary.skipped, 1, "one criterion should be skipped");
        assert_eq!(summary.results.len(), 3);
        assert!(summary.results[1].result.is_none());
    }

    #[test]
    fn evaluate_all_gates_equivalent_to_legacy() {
        let dir = tempfile::tempdir().unwrap();

        // Create equivalent specs in both formats
        let legacy_spec = Spec {
            name: "test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![Criterion {
                name: "echo".to_string(),
                description: "echo test".to_string(),
                cmd: Some("echo ok".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };

        let gates_spec = GatesSpec {
            name: "test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![GateCriterion {
                name: "echo".to_string(),
                description: "echo test".to_string(),
                cmd: Some("echo ok".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };

        let legacy_summary = evaluate_all(&legacy_spec, dir.path(), None, None);
        let gates_summary = evaluate_all_gates(&gates_spec, dir.path(), None, None);

        // Both return GateRunSummary with same structure
        assert_eq!(legacy_summary.spec_name, gates_summary.spec_name);
        assert_eq!(legacy_summary.passed, gates_summary.passed);
        assert_eq!(legacy_summary.failed, gates_summary.failed);
        assert_eq!(legacy_summary.skipped, gates_summary.skipped);
        assert_eq!(legacy_summary.results.len(), gates_summary.results.len());
        assert_eq!(
            legacy_summary.results[0].criterion_name,
            gates_summary.results[0].criterion_name
        );
    }

    #[test]
    fn to_criterion_drops_requirements() {
        let gc = GateCriterion {
            name: "test".to_string(),
            description: "desc".to_string(),
            cmd: Some("echo ok".to_string()),
            path: None,
            timeout: Some(60),
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec!["REQ-FUNC-001".to_string()],
        };

        let c = to_criterion(&gc);
        assert_eq!(c.name, "test");
        assert_eq!(c.description, "desc");
        assert_eq!(c.cmd, Some("echo ok".to_string()));
        assert_eq!(c.path, None);
        assert_eq!(c.timeout, Some(60));
    }

    #[test]
    fn to_criterion_preserves_path() {
        let gc = GateCriterion {
            name: "file-check".to_string(),
            description: "check file".to_string(),
            cmd: None,
            path: Some("dist/app.wasm".to_string()),
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let c = to_criterion(&gc);
        assert_eq!(c.path, Some("dist/app.wasm".to_string()));
        assert_eq!(c.cmd, None);
    }

    // ── FileExists dispatch via evaluate() ──────────────────────────

    #[test]
    fn evaluate_dispatches_file_exists_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.txt"), "content").unwrap();

        let criterion = Criterion {
            name: "file check".to_string(),
            description: "check file exists".to_string(),
            cmd: None,
            path: Some("target.txt".to_string()),
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        assert!(result.passed, "existing file should pass");
        assert!(matches!(result.kind, GateKind::FileExists { ref path } if path == "target.txt"));
    }

    #[test]
    fn evaluate_dispatches_file_exists_missing() {
        let dir = tempfile::tempdir().unwrap();

        let criterion = Criterion {
            name: "file check".to_string(),
            description: "check file exists".to_string(),
            cmd: None,
            path: Some("missing.txt".to_string()),
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        assert!(!result.passed, "missing file should fail");
        assert!(result.stderr.contains("file not found"));
    }

    #[test]
    fn evaluate_all_includes_file_exists_criteria() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("exists.txt"), "content").unwrap();

        let spec = Spec {
            name: "file-check".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                Criterion {
                    name: "file present".to_string(),
                    description: "checks file".to_string(),
                    cmd: None,
                    path: Some("exists.txt".to_string()),
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "descriptive only".to_string(),
                    description: "no cmd or path".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        assert_eq!(summary.passed, 1, "file exists criterion should pass");
        assert_eq!(
            summary.skipped, 1,
            "descriptive criterion should be skipped"
        );
        assert_eq!(summary.failed, 0);
    }

    #[test]
    fn evaluate_cmd_takes_precedence_over_path() {
        let dir = tempfile::tempdir().unwrap();

        let criterion = Criterion {
            name: "both set".to_string(),
            description: "cmd wins".to_string(),
            cmd: Some("echo cmd-ran".to_string()),
            path: Some("irrelevant.txt".to_string()),
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        assert!(result.passed);
        assert!(
            matches!(result.kind, GateKind::Command { .. }),
            "cmd should take precedence"
        );
    }

    #[test]
    fn evaluate_all_gates_with_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.md"), "# Hello").unwrap();

        let gates = GatesSpec {
            name: "file-gates".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![GateCriterion {
                name: "readme exists".to_string(),
                description: "check readme".to_string(),
                cmd: None,
                path: Some("readme.md".to_string()),
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };

        let summary = evaluate_all_gates(&gates, dir.path(), None, None);

        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);
        let result = summary.results[0].result.as_ref().unwrap();
        assert!(matches!(result.kind, GateKind::FileExists { ref path } if path == "readme.md"));
    }

    #[test]
    fn evaluate_file_exists_rejects_absolute_path() {
        let dir = tempfile::tempdir().unwrap();

        let result = evaluate_file_exists("/etc/passwd", dir.path()).unwrap();

        assert!(!result.passed);
        assert!(result.stderr.contains("path must be relative"));
    }

    // ── resolve_enforcement ─────────────────────────────────────────

    #[test]
    fn resolve_enforcement_precedence() {
        // None + None => Required (default)
        assert_eq!(resolve_enforcement(None, None), Enforcement::Required,);

        // None + Some(Advisory) => Advisory (gate section default)
        assert_eq!(
            resolve_enforcement(
                None,
                Some(&GateSection {
                    enforcement: Enforcement::Advisory
                })
            ),
            Enforcement::Advisory,
        );

        // Some(Required) + Some(Advisory) => Required (criterion override wins)
        assert_eq!(
            resolve_enforcement(
                Some(Enforcement::Required),
                Some(&GateSection {
                    enforcement: Enforcement::Advisory
                }),
            ),
            Enforcement::Required,
        );

        // Some(Advisory) + None => Advisory (criterion override wins)
        assert_eq!(
            resolve_enforcement(Some(Enforcement::Advisory), None),
            Enforcement::Advisory,
        );

        // None + Some(Required) => Required (gate section default)
        assert_eq!(
            resolve_enforcement(
                None,
                Some(&GateSection {
                    enforcement: Enforcement::Required
                })
            ),
            Enforcement::Required,
        );
    }

    // ── enforcement tracking in evaluate_all ─────────────────────────

    #[test]
    fn evaluate_all_advisory_failure_does_not_block() {
        let dir = tempfile::tempdir().unwrap();
        let spec = Spec {
            name: "advisory-test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                Criterion {
                    name: "required-pass".to_string(),
                    description: "passes".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Required),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "advisory-fail".to_string(),
                    description: "fails but advisory".to_string(),
                    cmd: Some("false".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Advisory),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        // Legacy counts: 1 pass, 1 fail
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);

        // Enforcement counts
        assert_eq!(summary.enforcement.required_passed, 1);
        assert_eq!(summary.enforcement.required_failed, 0);
        assert_eq!(summary.enforcement.advisory_passed, 0);
        assert_eq!(summary.enforcement.advisory_failed, 1);

        // CriterionResult enforcement fields
        assert_eq!(summary.results[0].enforcement, Enforcement::Required);
        assert_eq!(summary.results[1].enforcement, Enforcement::Advisory);
    }

    #[test]
    fn evaluate_all_skipped_excluded_from_enforcement() {
        let dir = tempfile::tempdir().unwrap();
        let spec = Spec {
            name: "skip-test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                Criterion {
                    name: "required-pass".to_string(),
                    description: "passes".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Required),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "descriptive-only".to_string(),
                    description: "no cmd or path".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Required),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        assert_eq!(summary.passed, 1);
        assert_eq!(summary.skipped, 1);

        // Enforcement summary should only count the executable criterion
        assert_eq!(summary.enforcement.required_passed, 1);
        assert_eq!(summary.enforcement.required_failed, 0);
        assert_eq!(summary.enforcement.advisory_passed, 0);
        assert_eq!(summary.enforcement.advisory_failed, 0);

        // Skipped criterion still has its resolved enforcement set
        assert_eq!(summary.results[1].enforcement, Enforcement::Required);
    }

    #[test]
    fn evaluate_all_gates_enforcement_tracking() {
        let dir = tempfile::tempdir().unwrap();
        let gates = GatesSpec {
            name: "enforcement-gates".to_string(),
            description: String::new(),
            gate: Some(GateSection {
                enforcement: Enforcement::Advisory,
            }),
            depends: vec![],
            criteria: vec![
                GateCriterion {
                    name: "required-override".to_string(),
                    description: "overrides to required".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Required),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                GateCriterion {
                    name: "inherits-advisory".to_string(),
                    description: "inherits advisory from gate section".to_string(),
                    cmd: Some("false".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };

        let summary = evaluate_all_gates(&gates, dir.path(), None, None);

        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.enforcement.required_passed, 1);
        assert_eq!(summary.enforcement.required_failed, 0);
        assert_eq!(summary.enforcement.advisory_passed, 0);
        assert_eq!(summary.enforcement.advisory_failed, 1);

        // Verify resolved enforcement on results
        assert_eq!(summary.results[0].enforcement, Enforcement::Required);
        assert_eq!(summary.results[1].enforcement, Enforcement::Advisory);
    }

    #[test]
    fn all_required_pass_advisory_failures_still_pass() {
        let dir = tempfile::tempdir().unwrap();
        let spec = Spec {
            name: "mixed-enforcement".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                Criterion {
                    name: "required-pass".to_string(),
                    description: "required, passes".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Required),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "advisory-fail".to_string(),
                    description: "advisory, fails".to_string(),
                    cmd: Some("false".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Advisory),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "advisory-pass".to_string(),
                    description: "advisory, passes".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Advisory),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        // Legacy: passed=2, failed=1
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);

        // Enforcement: all required pass, advisory mixed
        assert_eq!(summary.enforcement.required_passed, 1);
        assert_eq!(summary.enforcement.required_failed, 0);
        assert_eq!(summary.enforcement.advisory_passed, 1);
        assert_eq!(summary.enforcement.advisory_failed, 1);
    }

    // ── AgentReport dispatch ────────────────────────────────────────

    #[test]
    fn evaluate_agent_criterion_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "agent-review".to_string(),
            description: "Agent reviews code".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: Some(CriterionKind::AgentReport),
            prompt: Some("Review the auth module".to_string()),
            requirements: vec![],
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10));
        assert!(
            result.is_err(),
            "evaluate() should return error for AgentReport criteria"
        );

        let err = result.unwrap_err();
        let display = err.to_string();
        assert!(
            display.contains("agent-review"),
            "error should mention the criterion name, got: {display}"
        );
    }

    #[test]
    fn evaluate_all_with_agent_criterion_marks_as_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let spec = Spec {
            name: "mixed-with-agent".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                Criterion {
                    name: "cmd-pass".to_string(),
                    description: "command that passes".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Required),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
                Criterion {
                    name: "agent-review".to_string(),
                    description: "agent evaluates code quality".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Advisory),
                    kind: Some(CriterionKind::AgentReport),
                    prompt: Some("Check code quality".to_string()),
                    requirements: vec![],
                },
                Criterion {
                    name: "cmd-fail".to_string(),
                    description: "command that fails".to_string(),
                    cmd: Some("false".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: Some(Enforcement::Advisory),
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                },
            ],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        assert_eq!(summary.passed, 1, "one command should pass");
        assert_eq!(summary.failed, 1, "one command should fail");
        assert_eq!(summary.skipped, 1, "agent criterion should be skipped");
        assert_eq!(summary.results.len(), 3);

        // Agent criterion is skipped (result: None)
        let agent_result = &summary.results[1];
        assert_eq!(agent_result.criterion_name, "agent-review");
        assert!(
            agent_result.result.is_none(),
            "agent criterion should have no result (pending)"
        );
        assert_eq!(
            agent_result.enforcement,
            Enforcement::Advisory,
            "enforcement should still be resolved"
        );

        // Enforcement summary should NOT count the skipped agent criterion
        assert_eq!(summary.enforcement.required_passed, 1);
        assert_eq!(summary.enforcement.required_failed, 0);
        assert_eq!(summary.enforcement.advisory_passed, 0);
        assert_eq!(summary.enforcement.advisory_failed, 1);
    }

    #[test]
    fn evaluate_all_gates_captures_spawn_failure() {
        let dir = tempfile::tempdir().unwrap();
        let gates = GatesSpec {
            name: "bad-gates-cmd".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![GateCriterion {
                name: "impossible".to_string(),
                description: "nonexistent binary".to_string(),
                cmd: Some("/nonexistent/binary/that/does/not/exist".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
            }],
        };

        let summary = evaluate_all_gates(&gates, dir.path(), None, None);

        assert_eq!(
            summary.failed, 1,
            "spawn failure should count as failed in gates path"
        );
        let result = summary.results[0].result.as_ref().unwrap();
        assert!(!result.passed, "spawn failure should not pass");
        assert!(
            !result.stderr.is_empty(),
            "stderr should contain error message"
        );
    }

    // ── truncate_head_tail ──────────────────────────────────────────

    #[test]
    fn truncate_head_tail_within_budget() {
        let input = "short string";
        let result = truncate_head_tail(input.to_owned(), 1024);

        assert_eq!(result.output, input);
        assert!(!result.truncated);
        assert_eq!(result.original_bytes, input.len());
    }

    #[test]
    fn truncate_head_tail_exact_budget() {
        let input = "exactly this long";
        let result = truncate_head_tail(input.to_owned(), input.len());

        assert_eq!(result.output, input);
        assert!(!result.truncated);
        assert_eq!(result.original_bytes, input.len());
    }

    #[test]
    fn truncate_head_tail_over_budget() {
        let input = "a".repeat(300);
        let result = truncate_head_tail(input.clone(), 100);

        assert!(result.truncated);
        assert_eq!(result.original_bytes, 300);
        // The output should be shorter than the original
        // (head + marker + tail, where head+tail = budget)
        assert!(
            result.output.len() < input.len(),
            "truncated output should be shorter than input"
        );
        // Verify exact structure: marker present, omitted count = 300 - 100 = 200
        assert!(
            result.output.contains("[truncated: 200 bytes omitted]"),
            "marker should report exactly 200 bytes omitted, got: {:?}",
            result.output
        );
        // Head is 1/3 of budget = 33 bytes, tail = 67 bytes
        let marker_pos = result.output.find("[truncated: ").unwrap();
        let before_marker = result.output[..marker_pos].trim_end_matches('\n');
        let after_end = result.output.find(" bytes omitted]").unwrap() + " bytes omitted]".len();
        let after_marker = result.output[after_end..].trim_start_matches('\n');
        assert_eq!(before_marker.len(), 33, "head should be 33 bytes");
        assert_eq!(after_marker.len(), 67, "tail should be 67 bytes");
    }

    #[test]
    fn truncate_head_tail_marker_format() {
        let input = "a".repeat(300);
        let result = truncate_head_tail(input.clone(), 100);

        assert!(
            result.output.contains("[truncated: "),
            "output should contain '[truncated: ', got: {:?}",
            result.output
        );
        assert!(
            result.output.contains(" bytes omitted]"),
            "output should contain ' bytes omitted]', got: {:?}",
            result.output
        );
        // Marker must end with a newline before the tail content
        let marker_end = result.output.find(" bytes omitted]").unwrap() + " bytes omitted]".len();
        assert_eq!(
            result.output.as_bytes().get(marker_end).copied(),
            Some(b'\n'),
            "marker must be followed by a newline"
        );
        // Head must be preceded by a newline (or be at start)
        let marker_start = result.output.find("[truncated: ").unwrap();
        if marker_start > 0 {
            assert_eq!(
                result.output.as_bytes().get(marker_start - 1).copied(),
                Some(b'\n'),
                "marker must be preceded by a newline"
            );
        }
    }

    #[test]
    fn truncate_head_tail_preserves_head_and_tail() {
        // Use distinct head and tail content
        let head_content = "HEAD_START_xxxxx";
        let tail_content = "xxxxx_TAIL_END!!";
        let middle = "m".repeat(300);
        let input = format!("{head_content}{middle}{tail_content}");

        let result = truncate_head_tail(input.clone(), 100);

        assert!(result.truncated);
        // Head portion should appear before the marker
        let marker_pos = result.output.find("[truncated: ").unwrap();
        let before_marker = &result.output[..marker_pos];
        assert!(
            before_marker.contains("HEAD_START"),
            "head portion should contain start of input, got: {:?}",
            before_marker
        );
        // Tail portion should appear after the marker
        let after_marker = &result.output[marker_pos..];
        assert!(
            after_marker.contains("TAIL_END"),
            "tail portion should contain end of input, got: {:?}",
            after_marker
        );
    }

    #[test]
    fn truncate_head_tail_multiline_input() {
        // Test that multi-line input is handled correctly
        let lines: Vec<String> = (0..50).map(|i| format!("line {i}: content here")).collect();
        let input = lines.join("\n");
        let budget = 100;
        let result = truncate_head_tail(input.clone(), budget);

        assert!(result.truncated, "multi-line input over budget should be truncated");
        assert!(
            result.output.contains("[truncated: "),
            "truncated multi-line output should have marker"
        );
        // Head and tail bytes total == budget
        let marker_start = result.output.find("[truncated: ").unwrap();
        let marker_end =
            result.output.find(" bytes omitted]").unwrap() + " bytes omitted]".len();
        let before = result.output[..marker_start].trim_end_matches('\n');
        let after = result.output[marker_end..].trim_start_matches('\n');
        assert_eq!(
            before.len() + after.len(),
            budget,
            "head + tail should equal budget"
        );
    }

    #[test]
    fn truncate_head_tail_utf8_multibyte() {
        // '世' is 3 bytes in UTF-8
        let input = "世".repeat(100); // 300 bytes
        let result = truncate_head_tail(input.clone(), 100);

        assert!(result.truncated);
        // Must be valid UTF-8 (this would panic if not)
        let _ = result.output.as_str();
        // Verify no partial characters by checking all chars are '世' or marker chars
        for ch in result.output.chars() {
            assert!(
                ch == '世' || ch.is_ascii() || ch == '\n',
                "unexpected character in output: {:?}",
                ch
            );
        }
    }

    #[test]
    fn truncate_head_tail_utf8_4byte() {
        // '🦀' is 4 bytes in UTF-8
        let input = "🦀".repeat(100); // 400 bytes
        let result = truncate_head_tail(input.clone(), 100);

        assert!(result.truncated);
        // Must be valid UTF-8
        let _ = result.output.as_str();
        for ch in result.output.chars() {
            assert!(
                ch == '🦀' || ch.is_ascii() || ch == '\n',
                "unexpected character in output: {:?}",
                ch
            );
        }
    }

    #[test]
    fn truncate_head_tail_empty_input() {
        let result = truncate_head_tail(String::new(), 1024);

        assert_eq!(result.output, "");
        assert!(!result.truncated);
        assert_eq!(result.original_bytes, 0);
    }

    #[test]
    fn truncate_head_tail_overlap_guard() {
        // Use 4-byte emoji chars with a budget that causes head/tail overlap.
        // 4 emojis = 16 bytes, budget=15: head_budget=5, tail_budget=10
        // head_end = floor_char_boundary(5) = 4 (one emoji)
        // tail_start = ceil_char_boundary(16-10=6) = 8 (after second emoji)
        // No overlap with this split. Need smaller budget to force overlap.
        //
        // 3 emojis = 12 bytes, budget=11: head_budget=3, tail_budget=8
        // head_end = floor_char_boundary(3) = 0 (no complete emoji fits)
        // tail_start = ceil_char_boundary(12-8=4) = 4
        // 4 > 0, no overlap. The overlap path is very hard to hit with
        // aligned chars. Test with ASCII barely-over instead, which still
        // exercises the function without panic.
        let input = "a".repeat(105);
        let result = truncate_head_tail(input.clone(), 100);

        assert!(result.truncated);
        // Must not panic and must be valid UTF-8
        let _ = result.output.as_str();
        assert!(
            result.output.contains("[truncated: "),
            "should still have truncation marker"
        );

        // Also test budget=0 to exercise tail-only fallback
        let result_zero = truncate_head_tail("hello world".to_owned(), 0);
        assert!(result_zero.truncated);
        assert!(result_zero.output.contains("[truncated: 11 bytes omitted]"));
        // With budget=0: head_budget=0, tail_budget=0
        // head_end=0, tail_start=ceil_char_boundary(11-0=11)=11
        // tail_start(11) > head_end(0), so normal path: head="", tail=""
        // omitted=11, output is just the marker
    }

    #[test]
    fn truncate_head_tail_tiny_budget() {
        let input = "hello world, this is a test string";
        let result = truncate_head_tail(input.to_owned(), 1);

        assert!(result.truncated);
        // Must not panic and must be valid UTF-8
        let _ = result.output.as_str();
        assert!(
            result.output.contains("[truncated: "),
            "should have truncation marker"
        );
    }

    // ── truncate_diff ──────────────────────────────────────────────

    #[test]
    fn truncate_diff_empty_returns_none() {
        let (diff, truncated, original) = truncate_diff("", 1024);
        assert!(diff.is_none());
        assert!(!truncated);
        assert!(original.is_none());
    }

    #[test]
    fn truncate_diff_within_budget() {
        let input = "diff --git a/foo.rs b/foo.rs\n+hello\n";
        let (diff, truncated, original) = truncate_diff(input, 1024);
        assert_eq!(diff.as_deref(), Some(input));
        assert!(!truncated);
        assert_eq!(original, Some(input.len()));
    }

    #[test]
    fn truncate_diff_over_budget() {
        let input = "a".repeat(200);
        let (diff, truncated, original) = truncate_diff(&input, 50);
        assert!(diff.is_some());
        assert!(truncated);
        assert_eq!(original, Some(200));
        assert!(diff.unwrap().contains("[truncated: "));
    }

    // ── ERR-01: extract_binary ─────────────────────────────────────

    #[test]
    fn extract_binary_multi_word() {
        assert_eq!(extract_binary("cargo test --release"), "cargo");
    }

    #[test]
    fn extract_binary_single_word() {
        assert_eq!(extract_binary("sh"), "sh");
    }

    #[test]
    fn extract_binary_empty() {
        assert_eq!(extract_binary(""), "");
    }

    #[test]
    fn extract_binary_leading_whitespace() {
        assert_eq!(extract_binary("  cargo test"), "cargo");
    }

    // ── ERR-01: classify_exit_code ─────────────────────────────────

    #[test]
    fn classify_exit_code_127_is_not_found() {
        assert!(matches!(
            classify_exit_code(127),
            Some(CommandErrorKind::NotFound)
        ));
    }

    #[test]
    fn classify_exit_code_126_is_not_executable() {
        assert!(matches!(
            classify_exit_code(126),
            Some(CommandErrorKind::NotExecutable)
        ));
    }

    #[test]
    fn classify_exit_code_1_is_none() {
        assert!(classify_exit_code(1).is_none());
    }

    #[test]
    fn classify_exit_code_0_is_none() {
        assert!(classify_exit_code(0).is_none());
    }

    // ── ERR-01: format_command_error ───────────────────────────────

    #[test]
    fn format_command_error_not_found() {
        let msg = format_command_error("cargo test", CommandErrorKind::NotFound);
        assert!(
            msg.contains("cargo"),
            "should contain binary name, got: {msg}"
        );
        assert!(
            msg.contains("not found"),
            "should say not found, got: {msg}"
        );
        assert!(msg.contains("PATH"), "should mention PATH, got: {msg}");
    }

    #[test]
    fn format_command_error_not_executable() {
        let msg = format_command_error("cargo test", CommandErrorKind::NotExecutable);
        assert!(
            msg.contains("cargo"),
            "should contain binary name, got: {msg}"
        );
        assert!(
            msg.contains("not executable"),
            "should say not executable, got: {msg}"
        );
        assert!(
            msg.contains("permissions"),
            "should mention permissions, got: {msg}"
        );
    }

    #[test]
    fn format_command_error_not_found_single_word() {
        let msg = format_command_error("sh", CommandErrorKind::NotFound);
        assert!(msg.contains("sh"), "should contain binary name, got: {msg}");
        assert!(
            msg.contains("not found"),
            "should say not found, got: {msg}"
        );
    }

    // ── enriched_error_display tests ────────────────────────────────

    #[test]
    fn enriched_error_display_not_found_with_cmd() {
        let err = AssayError::GateExecution {
            cmd: "cargo test".to_string(),
            working_dir: std::path::PathBuf::from("/tmp"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = enriched_error_display(&err, Some("cargo test"));
        assert!(
            msg.contains("not found"),
            "should contain not-found hint, got: {msg}"
        );
        assert!(msg.contains("PATH"), "should contain PATH hint, got: {msg}");
    }

    #[test]
    fn enriched_error_display_permission_denied_with_cmd() {
        let err = AssayError::GateExecution {
            cmd: "cargo test".to_string(),
            working_dir: std::path::PathBuf::from("/tmp"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied"),
        };
        let msg = enriched_error_display(&err, Some("cargo test"));
        assert!(
            msg.contains("not executable"),
            "should contain permission hint, got: {msg}"
        );
        assert!(
            msg.contains("permissions"),
            "should mention permissions, got: {msg}"
        );
    }

    #[test]
    fn enriched_error_display_other_io_error_with_cmd() {
        let err = AssayError::GateExecution {
            cmd: "cargo test".to_string(),
            working_dir: std::path::PathBuf::from("/tmp"),
            source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe"),
        };
        let msg = enriched_error_display(&err, Some("cargo test"));
        assert!(
            msg.starts_with("gate evaluation error:"),
            "should be plain error message, got: {msg}"
        );
        assert!(
            !msg.contains("PATH"),
            "should not contain hint for other IO errors, got: {msg}"
        );
    }

    #[test]
    fn enriched_error_display_non_gate_execution_error() {
        let err = AssayError::SpecNotFound {
            name: "missing".to_string(),
            specs_dir: std::path::PathBuf::from("/tmp/specs"),
        };
        let msg = enriched_error_display(&err, Some("cargo test"));
        assert!(
            msg.starts_with("gate evaluation error:"),
            "should be plain error message, got: {msg}"
        );
        assert!(
            !msg.contains("PATH"),
            "should not contain hint for non-GateExecution errors, got: {msg}"
        );
    }

    #[test]
    fn enriched_error_display_gate_execution_no_cmd() {
        let err = AssayError::GateExecution {
            cmd: "cargo test".to_string(),
            working_dir: std::path::PathBuf::from("/tmp"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = enriched_error_display(&err, None);
        assert!(
            msg.starts_with("gate evaluation error:"),
            "should be plain error message, got: {msg}"
        );
        assert!(
            !msg.contains("PATH"),
            "should not contain hint when cmd is None, got: {msg}"
        );
    }
}
