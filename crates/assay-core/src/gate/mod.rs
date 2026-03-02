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
use serde::Serialize;

use assay_types::{Criterion, GateKind, GateResult, Spec};

use crate::error::{AssayError, Result};

/// Maximum bytes to retain from stdout/stderr capture (64 KB).
const MAX_OUTPUT_BYTES: usize = 65_536;

/// Polling interval for `try_wait` timeout loop.
const POLL_INTERVAL_MS: u64 = 50;

/// Minimum timeout floor in seconds.
const MIN_TIMEOUT_SECS: u64 = 1;

/// Summary of evaluating all criteria in a spec.
///
/// This is a computed summary type (not a persisted DTO). It lives in
/// `assay-core`, not `assay-types`, because it represents aggregate
/// evaluation results rather than configuration or state.
#[derive(Debug, Clone, Serialize)]
pub struct GateRunSummary {
    /// Spec name that was evaluated.
    pub spec_name: String,
    /// Results for each criterion that was evaluated or skipped.
    pub results: Vec<CriterionResult>,
    /// Number of criteria that passed.
    pub passed: usize,
    /// Number of criteria that failed.
    pub failed: usize,
    /// Number of criteria skipped (descriptive-only, no cmd).
    pub skipped: usize,
    /// Total wall-clock duration for all evaluations in milliseconds.
    pub total_duration_ms: u64,
}

/// A criterion paired with its evaluation result.
#[derive(Debug, Clone, Serialize)]
pub struct CriterionResult {
    /// The name of the criterion that was evaluated.
    pub criterion_name: String,
    /// The gate result, or `None` if skipped (no cmd).
    pub result: Option<GateResult>,
}

/// Evaluate a single criterion as a gate.
///
/// Derives `GateKind` from criterion fields: if `cmd` is `Some`, it's
/// `GateKind::Command`; if `cmd` is `None`, it's `GateKind::AlwaysPass`.
///
/// `working_dir` is required — this function never inherits the process CWD.
/// `timeout` is the maximum wall-clock time for the command to complete.
///
/// # Async Usage
///
/// This function is synchronous. From async code, use:
/// ```ignore
/// tokio::task::spawn_blocking(move || gate::evaluate(&criterion, &dir, timeout)).await?
/// ```
pub fn evaluate(
    criterion: &Criterion,
    working_dir: &Path,
    timeout: Duration,
) -> Result<GateResult> {
    match &criterion.cmd {
        Some(cmd) => evaluate_command(cmd, working_dir, timeout),
        None => evaluate_always_pass(),
    }
}

/// Evaluate all criteria in a spec sequentially.
///
/// Skips criteria without `cmd` (records as skipped in summary). Uses
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
    let start = Instant::now();
    let mut results = Vec::with_capacity(spec.criteria.len());
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for criterion in &spec.criteria {
        if criterion.cmd.is_none() {
            skipped += 1;
            results.push(CriterionResult {
                criterion_name: criterion.name.clone(),
                result: None,
            });
            continue;
        }

        let timeout = resolve_timeout(cli_timeout, criterion.timeout, config_timeout);

        match evaluate(criterion, working_dir, timeout) {
            Ok(gate_result) => {
                if gate_result.passed {
                    passed += 1;
                } else {
                    failed += 1;
                }
                results.push(CriterionResult {
                    criterion_name: criterion.name.clone(),
                    result: Some(gate_result),
                });
            }
            Err(err) => {
                failed += 1;
                results.push(CriterionResult {
                    criterion_name: criterion.name.clone(),
                    result: Some(GateResult {
                        passed: false,
                        kind: GateKind::Command {
                            cmd: criterion.cmd.clone().unwrap_or_default(),
                        },
                        stdout: String::new(),
                        stderr: format!("gate evaluation error: {err}"),
                        exit_code: None,
                        duration_ms: 0,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                    }),
                });
            }
        }
    }

    GateRunSummary {
        spec_name: spec.name.clone(),
        results,
        passed,
        failed,
        skipped,
        total_duration_ms: start.elapsed().as_millis() as u64,
    }
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
pub fn evaluate_file_exists(path: &str, working_dir: &Path) -> Result<GateResult> {
    let start = Instant::now();
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

    // Spawn reader threads to drain pipes (prevents deadlock)
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stdout) = stdout_handle {
            let _ = std::io::Read::read_to_end(&mut stdout, &mut buf);
        }
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stderr) = stderr_handle {
            let _ = std::io::Read::read_to_end(&mut stderr, &mut buf);
        }
        buf
    });

    // Poll for completion with timeout
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // Reap zombie
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

    // Join reader threads (safe: process is dead, pipes will EOF)
    let stdout_bytes = stdout_thread.join().unwrap_or_default();
    let stderr_bytes = stderr_thread.join().unwrap_or_default();

    let stdout_raw = String::from_utf8_lossy(&stdout_bytes);
    let stderr_raw = String::from_utf8_lossy(&stderr_bytes);

    // Track original sizes for truncation metadata
    let original_stdout_len = stdout_raw.len();
    let original_stderr_len = stderr_raw.len();

    // Apply truncation (tail-biased)
    let (stdout_str, stdout_truncated) = truncate_output(&stdout_raw, MAX_OUTPUT_BYTES);
    let (stderr_str, stderr_truncated) = truncate_output(&stderr_raw, MAX_OUTPUT_BYTES);

    let truncated = stdout_truncated || stderr_truncated;
    let original_bytes = if truncated {
        Some((original_stdout_len + original_stderr_len) as u64)
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
                stdout: stdout_str,
                stderr: stderr_str,
                exit_code,
                duration_ms,
                timestamp: Utc::now(),
                truncated,
                original_bytes,
            })
        }
        None => {
            // Timeout: append timeout notice to stderr
            let timeout_stderr = if stderr_str.is_empty() {
                format!("[timed out after {}s]", timeout.as_secs())
            } else {
                format!("{}\n[timed out after {}s]", stderr_str, timeout.as_secs())
            };

            Ok(GateResult {
                passed: false,
                kind: GateKind::Command {
                    cmd: cmd.to_string(),
                },
                stdout: stdout_str,
                stderr: timeout_stderr,
                exit_code: None,
                duration_ms,
                timestamp: Utc::now(),
                truncated,
                original_bytes,
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
    })
}

/// Truncate output, keeping the tail (since errors appear at end).
///
/// Returns `(possibly_truncated_string, was_truncated)`. If within
/// budget, returns the input unchanged. Otherwise, keeps the last
/// `max_bytes` bytes with a prepended truncation indicator, using
/// `str::ceil_char_boundary` for safe UTF-8 slicing.
fn truncate_output(output: &str, max_bytes: usize) -> (String, bool) {
    if output.len() <= max_bytes {
        return (output.to_string(), false);
    }
    let skip = output.len() - max_bytes;
    let start = output.ceil_char_boundary(skip);
    let truncated = format!(
        "[truncated, showing last {} bytes]\n{}",
        output.len() - start,
        &output[start..]
    );
    (truncated, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── evaluate: command execution ────────────────────────────────

    #[test]
    fn evaluate_echo_hello() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "echo test".to_string(),
            description: "runs echo".to_string(),
            cmd: Some("echo hello".to_string()),
            timeout: None,
        };

        let result = evaluate(&criterion, dir.path(), Duration::from_secs(10)).unwrap();

        assert!(result.passed, "echo should pass");
        assert!(
            result.stdout.contains("hello"),
            "stdout should contain 'hello', got: {:?}",
            result.stdout
        );
        assert_eq!(result.exit_code, Some(0));
        // duration_ms is populated (may be 0 on very fast machines)
        assert!(matches!(result.kind, GateKind::Command { .. }));
        assert!(matches!(result.kind, GateKind::Command { .. }));
    }

    #[test]
    fn evaluate_failing_command() {
        let dir = tempfile::tempdir().unwrap();
        let criterion = Criterion {
            name: "fail test".to_string(),
            description: "runs failing cmd".to_string(),
            cmd: Some("sh -c 'echo fail >&2 && exit 1'".to_string()),
            timeout: None,
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
            timeout: None,
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
            timeout: None,
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
            timeout: None,
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

    // ── truncate_output ────────────────────────────────────────────

    #[test]
    fn truncate_output_within_budget() {
        let input = "short string";
        let (output, was_truncated) = truncate_output(input, 1024);

        assert_eq!(output, input);
        assert!(!was_truncated);
    }

    #[test]
    fn truncate_output_over_budget() {
        let input = "a".repeat(200);
        let (output, was_truncated) = truncate_output(&input, 100);

        assert!(was_truncated);
        assert!(
            output.contains("[truncated, showing last"),
            "should have truncation indicator, got: {:?}",
            output
        );
        // The output should contain the tail portion
        assert!(output.contains(&"a".repeat(100)));
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
            criteria: vec![
                Criterion {
                    name: "passes".to_string(),
                    description: "will pass".to_string(),
                    cmd: Some("true".to_string()),
                    timeout: None,
                },
                Criterion {
                    name: "descriptive".to_string(),
                    description: "no cmd".to_string(),
                    cmd: None,
                    timeout: None,
                },
                Criterion {
                    name: "fails".to_string(),
                    description: "will fail".to_string(),
                    cmd: Some("false".to_string()),
                    timeout: None,
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
            criteria: vec![Criterion {
                name: "impossible".to_string(),
                description: "nonexistent binary".to_string(),
                cmd: Some("/nonexistent/binary/that/does/not/exist".to_string()),
                timeout: None,
            }],
        };

        let summary = evaluate_all(&spec, dir.path(), None, None);

        assert_eq!(summary.failed, 1, "spawn failure should count as failed");
        assert_eq!(summary.passed, 0);
        let result = summary.results[0].result.as_ref().unwrap();
        assert!(!result.passed);
    }
}
