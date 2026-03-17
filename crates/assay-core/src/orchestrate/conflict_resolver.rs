//! AI-driven conflict resolution via Claude Code subprocess.
//!
//! Spawns a sync `claude -p --json-schema` subprocess to resolve git merge
//! conflicts. Reads conflicted files from a live working tree, constructs a
//! structured prompt, parses the JSON response, writes resolved contents,
//! stages and commits. On any failure, returns `ConflictAction::Skip`.
//!
//! # Architecture
//!
//! ```text
//! build_conflict_prompt() → stdin ─→ [claude -p --json-schema ...] ─→ stdout
//!                                                                        │
//!                                       parse ConflictResolutionOutput ←─┘
//!                                                    │
//!                              write files → git add → git commit → Resolved(sha)
//! ```

use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use assay_types::{
    ConflictAction, ConflictFileContent, ConflictResolution, ConflictResolutionConfig, ConflictScan,
};

use crate::merge::git_command;

// ── Result type ──────────────────────────────────────────────────────

/// Result of a single conflict resolution attempt.
///
/// Bundles the [`ConflictAction`] decision together with the optional audit
/// record and a flag indicating whether the working tree was left clean.
/// Used by the merge runner (T02+) to record audit data in [`MergeReport`].
#[derive(Debug)]
pub struct ConflictResolutionResult {
    /// The resolution decision: `Resolved(sha)`, `Skip`, or `Abort`.
    pub action: ConflictAction,
    /// Full audit record when the AI successfully resolved the conflict.
    ///
    /// `None` when the resolver failed or chose `Skip`/`Abort` before producing output.
    pub audit: Option<ConflictResolution>,
    /// Whether the repository working tree and index are clean after resolution.
    ///
    /// `true` after a successful resolution commit; `false` if the resolver
    /// aborted mid-way and the caller needs to `git reset --hard`.
    pub repo_clean: bool,
}

// ── Response type ────────────────────────────────────────────────────

/// AI-generated resolution output: resolved file contents.
///
/// This is the `--json-schema` contract for the Claude subprocess.
/// Each `ResolvedFile` contains the full resolved content for one
/// conflicted file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ConflictResolutionOutput {
    /// Resolved file contents, one per conflicted file.
    pub resolved_files: Vec<ResolvedFile>,
}

/// A single file with its resolved content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedFile {
    /// Path relative to the repository root.
    pub path: String,
    /// Full resolved file content (conflict markers removed).
    pub content: String,
}

// ── Schema cache ─────────────────────────────────────────────────────

/// Cached JSON Schema string for `ConflictResolutionOutput`.
///
/// Generated once at first access; subsequent calls return the same allocation.
static CONFLICT_RESOLUTION_SCHEMA: LazyLock<String> = LazyLock::new(|| {
    let schema = schemars::schema_for!(ConflictResolutionOutput);
    serde_json::to_string(&schema).expect("schema serialization cannot fail")
});

/// Return the JSON Schema string for `ConflictResolutionOutput`.
pub fn conflict_resolution_schema_json() -> &'static str {
    &CONFLICT_RESOLUTION_SCHEMA
}

// ── Prompt construction ──────────────────────────────────────────────

/// Build the system prompt for the conflict resolver subprocess.
///
/// Instructs the AI on behavior and output expectations.
pub fn build_conflict_system_prompt() -> String {
    "You are resolving git merge conflicts. Your task is to produce the correct \
     resolved content for each conflicted file.\n\n\
     Rules:\n\
     - Resolve conflicts by choosing the best combination of both sides.\n\
     - Preserve all non-conflicting code exactly as-is.\n\
     - Do not introduce new code beyond what is needed to resolve the conflict.\n\
     - Do not add comments explaining the resolution.\n\
     - Remove all conflict markers (<<<<<<< , =======, >>>>>>>).\n\
     - Return the complete file content for each resolved file."
        .to_string()
}

/// Build the user prompt for the conflict resolver subprocess.
///
/// Reads the full content of each conflicted file from the working directory
/// (they contain conflict markers) and constructs a structured prompt.
pub fn build_conflict_prompt(
    session_name: &str,
    conflicting_files: &[String],
    conflict_scan: &ConflictScan,
    work_dir: &Path,
) -> String {
    let mut sections = Vec::new();

    // Context section
    sections.push(format!(
        "## Merge Conflict Resolution\n\n\
         Session: {session_name}\n\
         Number of conflicted files: {}",
        conflicting_files.len()
    ));

    // Conflict scan summary
    if !conflict_scan.markers.is_empty() {
        let mut scan_text = String::from("## Conflict Marker Summary\n");
        for marker in &conflict_scan.markers {
            scan_text.push_str(&format!(
                "\n- {}: line {} ({})",
                marker.file, marker.line, marker.marker_type
            ));
        }
        sections.push(scan_text);
    }

    // File contents section — read each conflicted file from the working tree
    let mut files_text = String::from("## Conflicted Files\n");
    for file_path in conflicting_files {
        let full_path = work_dir.join(file_path);
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => format!("(error reading file: {e})"),
        };
        files_text.push_str(&format!(
            "\n### File: `{file_path}`\n\n```\n{content}\n```\n"
        ));
    }
    sections.push(files_text);

    // Final instruction
    sections.push(
        "Resolve all conflicts in the files above. Return the complete resolved \
         content for each file."
            .to_string(),
    );

    sections.join("\n\n")
}

// ── Subprocess execution ─────────────────────────────────────────────

/// Run a shell validation command synchronously with the given timeout.
///
/// Uses `sh -c "<cmd>"` when the command contains spaces; otherwise invokes
/// the binary directly. Returns `Ok(())` on zero exit, `Err(message)` on
/// non-zero exit, timeout, or binary-not-found.
fn run_validation_command(
    cmd: &str,
    work_dir: &Path,
    timeout_secs: u64,
) -> std::result::Result<(), String> {
    let mut child = if cmd.contains(' ') {
        Command::new("sh")
            .args(["-c", cmd])
            .current_dir(work_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("validation command not found: {e}"))?
    } else {
        Command::new(cmd)
            .current_dir(work_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    format!("validation command not found: {cmd}")
                } else {
                    format!("validation command not found: {e}")
                }
            })?
    };

    let timeout = Duration::from_secs(timeout_secs);
    let output = wait_with_timeout(&mut child, timeout).inspect_err(|_| {
        let _ = child.kill();
        let _ = child.wait();
    })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "validation command exited with code {:?}",
            output.status.code()
        ))
    }
}

/// Resolve merge conflicts using an AI subprocess.
///
/// 1. Captures original file contents (with conflict markers) for the audit record.
/// 2. Builds a structured prompt from the conflicted files.
/// 3. Spawns `claude -p --json-schema` synchronously via `std::process::Command`.
/// 4. Parses the JSON response into `ConflictResolutionOutput`.
/// 5. Writes resolved file contents, stages with `git add`, commits.
/// 6. Optionally runs a validation command; on failure, rolls back with
///    `git reset --hard HEAD~1` and returns `repo_clean: true`.
/// 7. Returns `ConflictResolutionResult` with full audit data on success.
///
/// On any error before the commit, returns `repo_clean: false` (merge runner
/// must call `git merge --abort`). On validation failure with successful
/// rollback, returns `repo_clean: true`.
pub fn resolve_conflict(
    session_name: &str,
    conflicting_files: &[String],
    conflict_scan: &ConflictScan,
    work_dir: &Path,
    config: &ConflictResolutionConfig,
) -> ConflictResolutionResult {
    // Capture original file contents before any modification
    let original_contents: Vec<ConflictFileContent> = conflicting_files
        .iter()
        .map(|path| {
            let full_path = work_dir.join(path);
            let content = std::fs::read_to_string(&full_path)
                .unwrap_or_else(|e| format!("(error reading file: {e})"));
            ConflictFileContent {
                path: path.clone(),
                content,
            }
        })
        .collect();

    let prompt = build_conflict_prompt(session_name, conflicting_files, conflict_scan, work_dir);
    let system_prompt = build_conflict_system_prompt();
    let schema_json = conflict_resolution_schema_json();
    let timeout = Duration::from_secs(config.timeout_secs);

    // Spawn the Claude subprocess synchronously
    let resolver_stdout =
        match spawn_resolver(&prompt, &system_prompt, schema_json, &config.model, timeout) {
            Ok(output) => output,
            Err(err) => {
                tracing::warn!(
                    session_name,
                    error = %err,
                    "conflict resolver subprocess failed"
                );
                return ConflictResolutionResult {
                    action: ConflictAction::Skip,
                    audit: None,
                    repo_clean: false,
                };
            }
        };

    // Parse the structured output
    let resolution = match parse_resolver_output(&resolver_stdout) {
        Ok(resolution) => resolution,
        Err(err) => {
            tracing::warn!(
                session_name,
                error = %err,
                raw_output_len = resolver_stdout.len(),
                "conflict resolver output parse failed"
            );
            return ConflictResolutionResult {
                action: ConflictAction::Skip,
                audit: None,
                repo_clean: false,
            };
        }
    };

    // Collect resolved contents for the audit record (before writing to disk)
    let resolved_contents: Vec<ConflictFileContent> = resolution
        .resolved_files
        .iter()
        .map(|rf| ConflictFileContent {
            path: rf.path.clone(),
            content: rf.content.clone(),
        })
        .collect();

    // Write resolved files and stage them
    for resolved_file in &resolution.resolved_files {
        let file_path = work_dir.join(&resolved_file.path);
        if let Err(e) = std::fs::write(&file_path, &resolved_file.content) {
            tracing::warn!(
                session_name,
                path = %resolved_file.path,
                error = %e,
                "failed to write resolved file"
            );
            return ConflictResolutionResult {
                action: ConflictAction::Skip,
                audit: None,
                repo_clean: false,
            };
        }

        if let Err(e) = git_command(&["add", &resolved_file.path], work_dir) {
            tracing::warn!(
                session_name,
                path = %resolved_file.path,
                error = %e,
                "failed to git add resolved file"
            );
            return ConflictResolutionResult {
                action: ConflictAction::Skip,
                audit: None,
                repo_clean: false,
            };
        }
    }

    // Commit — MERGE_HEAD is present, so this creates a proper merge commit
    match git_command(&["commit", "--no-edit"], work_dir) {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(
                session_name,
                error = %e,
                "failed to commit conflict resolution"
            );
            return ConflictResolutionResult {
                action: ConflictAction::Skip,
                audit: None,
                repo_clean: false,
            };
        }
    }

    // Get the commit SHA
    let sha = match git_command(&["rev-parse", "HEAD"], work_dir) {
        Ok(sha) => sha,
        Err(e) => {
            tracing::warn!(
                session_name,
                error = %e,
                "conflict committed but failed to read HEAD SHA"
            );
            return ConflictResolutionResult {
                action: ConflictAction::Skip,
                audit: None,
                repo_clean: false,
            };
        }
    };

    // Run optional validation command after commit
    // NOTE: MERGE_HEAD is consumed by the commit, so rollback must use
    // `git reset --hard HEAD~1` — `git merge --abort` would fail here.
    if let Some(validation_cmd) = &config.validation_command {
        match run_validation_command(validation_cmd, work_dir, config.timeout_secs) {
            Ok(()) => {
                tracing::info!(
                    session_name,
                    sha = %sha,
                    validation_cmd = %validation_cmd,
                    validation_passed = true,
                    resolved_files = resolution.resolved_files.len(),
                    "conflict resolved successfully with validation"
                );
            }
            Err(reason) => {
                tracing::warn!(
                    session_name,
                    validation_cmd = %validation_cmd,
                    reason = %reason,
                    "validation command failed — rolling back merge commit"
                );
                // Rollback the merge commit. On reset failure, propagate as a hard error.
                match git_command(&["reset", "--hard", "HEAD~1"], work_dir) {
                    Ok(_) => {
                        return ConflictResolutionResult {
                            action: ConflictAction::Skip,
                            audit: None,
                            repo_clean: true,
                        };
                    }
                    Err(reset_err) => {
                        tracing::warn!(
                            session_name,
                            error = %reset_err,
                            "git reset --hard HEAD~1 failed after validation failure"
                        );
                        // Hard error: the repo is in an unknown state
                        return ConflictResolutionResult {
                            action: ConflictAction::Abort,
                            audit: None,
                            repo_clean: false,
                        };
                    }
                }
            }
        }
    } else {
        tracing::info!(
            session_name,
            sha = %sha,
            resolved_files = resolution.resolved_files.len(),
            "conflict resolved successfully"
        );
    }

    ConflictResolutionResult {
        action: ConflictAction::Resolved(sha),
        audit: Some(ConflictResolution {
            session_name: session_name.to_string(),
            conflicting_files: conflicting_files.to_vec(),
            original_contents,
            resolved_contents,
            resolver_stdout,
            validation_passed: config.validation_command.as_ref().map(|_| true),
        }),
        repo_clean: false,
    }
}

/// Spawn the Claude CLI subprocess synchronously and collect stdout.
///
/// Returns the raw stdout on success, or a descriptive error string on failure.
fn spawn_resolver(
    prompt: &str,
    system_prompt: &str,
    schema_json: &str,
    model: &str,
    timeout: Duration,
) -> std::result::Result<String, String> {
    use std::io::Write;

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
            model,
            "--no-session-persistence",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "claude CLI not found in PATH".to_string()
            } else {
                format!("failed to spawn claude: {e}")
            }
        })?;

    // Write prompt to stdin, then close it
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .map_err(|e| format!("stdin write failed: {e}"))?;
        // stdin dropped here, sending EOF
    }

    // Wait with timeout using a background thread
    let start = std::time::Instant::now();
    let output = wait_with_timeout(&mut child, timeout).map_err(|e| {
        // Kill the child on timeout
        let _ = child.kill();
        let _ = child.wait();
        format!("{e} (elapsed: {:.1}s)", start.elapsed().as_secs_f64())
    })?;

    let exit_code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let stderr_snippet = if stderr.len() > 200 {
            format!("{}...", &stderr[..200])
        } else {
            stderr
        };
        return Err(format!(
            "claude exited with code {exit_code:?}: {stderr_snippet}"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Wait for a child process with a timeout.
///
/// Uses `child.try_wait()` polling with sleep since `std::process::Child`
/// doesn't support native timeouts.
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> std::result::Result<std::process::Output, String> {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited — collect stdout/stderr
                use std::io::Read;
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(ref mut out) = child.stdout {
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(ref mut err) = child.stderr {
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                // Still running
                if start.elapsed() > timeout {
                    return Err(format!("resolver timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(format!("error waiting for process: {e}"));
            }
        }
    }
}

/// Parse Claude CLI JSON output into `ConflictResolutionOutput`.
///
/// Handles the Claude Code envelope format: extracts `structured_output`
/// from the top-level JSON object.
fn parse_resolver_output(stdout: &str) -> std::result::Result<ConflictResolutionOutput, String> {
    let envelope: serde_json::Value =
        serde_json::from_str(stdout).map_err(|e| format!("invalid JSON: {e}"))?;

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
        return Err(format!("claude reported error: {result}"));
    }

    // Extract structured_output
    let structured = envelope
        .get("structured_output")
        .ok_or("missing structured_output field in response")?;

    serde_json::from_value(structured.clone())
        .map_err(|e| format!("structured_output parse error: {e}"))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{ConflictMarker, MarkerType};

    // ── Schema generation ──────────────────────────────────────────

    #[test]
    fn conflict_resolution_schema_is_valid_json() {
        let json = conflict_resolution_schema_json();
        assert!(!json.is_empty());
        let value: serde_json::Value =
            serde_json::from_str(json).expect("schema should be valid JSON");
        assert!(value.is_object());
        // Should reference our type's fields
        assert!(json.contains("resolved_files"));
        assert!(json.contains("path"));
        assert!(json.contains("content"));
    }

    // ── Response type deserialization ──────────────────────────────

    #[test]
    fn conflict_resolution_output_deserialize_valid() {
        let json = serde_json::json!({
            "resolved_files": [
                {
                    "path": "src/main.rs",
                    "content": "fn main() {\n    println!(\"resolved\");\n}\n"
                },
                {
                    "path": "src/lib.rs",
                    "content": "pub fn lib() {}\n"
                }
            ]
        });

        let output: ConflictResolutionOutput = serde_json::from_value(json).unwrap();
        assert_eq!(output.resolved_files.len(), 2);
        assert_eq!(output.resolved_files[0].path, "src/main.rs");
        assert!(output.resolved_files[0].content.contains("resolved"));
        assert_eq!(output.resolved_files[1].path, "src/lib.rs");
    }

    #[test]
    fn conflict_resolution_output_deserialize_empty_files() {
        let json = serde_json::json!({
            "resolved_files": []
        });

        let output: ConflictResolutionOutput = serde_json::from_value(json).unwrap();
        assert!(output.resolved_files.is_empty());
    }

    #[test]
    fn conflict_resolution_output_deserialize_malformed_missing_field() {
        let json = serde_json::json!({
            "wrong_field": []
        });

        let result = serde_json::from_value::<ConflictResolutionOutput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn conflict_resolution_output_deserialize_malformed_bad_item() {
        let json = serde_json::json!({
            "resolved_files": [
                { "path": "file.rs" }
            ]
        });

        let result = serde_json::from_value::<ConflictResolutionOutput>(json);
        assert!(result.is_err(), "missing 'content' field should fail");
    }

    #[test]
    fn conflict_resolution_output_serde_roundtrip() {
        let output = ConflictResolutionOutput {
            resolved_files: vec![ResolvedFile {
                path: "test.rs".to_string(),
                content: "fn test() {}\n".to_string(),
            }],
        };
        let json = serde_json::to_string(&output).unwrap();
        let back: ConflictResolutionOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(back, output);
    }

    // ── parse_resolver_output ──────────────────────────────────────

    #[test]
    fn parse_resolver_output_valid_envelope() {
        let stdout = serde_json::json!({
            "result": "text",
            "structured_output": {
                "resolved_files": [
                    { "path": "file.rs", "content": "resolved content" }
                ]
            },
            "is_error": false
        })
        .to_string();

        let output = parse_resolver_output(&stdout).unwrap();
        assert_eq!(output.resolved_files.len(), 1);
        assert_eq!(output.resolved_files[0].path, "file.rs");
    }

    #[test]
    fn parse_resolver_output_is_error_flag() {
        let stdout = serde_json::json!({
            "is_error": true,
            "result": "rate limit exceeded"
        })
        .to_string();

        let err = parse_resolver_output(&stdout).unwrap_err();
        assert!(err.contains("rate limit exceeded"));
    }

    #[test]
    fn parse_resolver_output_missing_structured_output() {
        let stdout = serde_json::json!({
            "result": "text only"
        })
        .to_string();

        let err = parse_resolver_output(&stdout).unwrap_err();
        assert!(err.contains("missing structured_output"));
    }

    #[test]
    fn parse_resolver_output_invalid_json() {
        let err = parse_resolver_output("not json").unwrap_err();
        assert!(err.contains("invalid JSON"));
    }

    #[test]
    fn parse_resolver_output_bad_structured_output() {
        let stdout = serde_json::json!({
            "structured_output": { "wrong": true }
        })
        .to_string();

        let err = parse_resolver_output(&stdout).unwrap_err();
        assert!(err.contains("structured_output parse error"));
    }

    // ── Prompt construction ────────────────────────────────────────

    #[test]
    fn build_conflict_prompt_includes_session_name() {
        let prompt = build_conflict_prompt(
            "auth-flow",
            &["file.rs".to_string()],
            &ConflictScan {
                has_markers: false,
                markers: vec![],
                truncated: false,
            },
            Path::new("/nonexistent"),
        );
        assert!(prompt.contains("auth-flow"));
    }

    #[test]
    fn build_conflict_prompt_includes_file_count() {
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let prompt = build_conflict_prompt(
            "test",
            &files,
            &ConflictScan {
                has_markers: false,
                markers: vec![],
                truncated: false,
            },
            Path::new("/nonexistent"),
        );
        assert!(prompt.contains("2"));
    }

    #[test]
    fn build_conflict_prompt_includes_file_contents_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let content = "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> branch\n";
        std::fs::write(dir.path().join("conflict.rs"), content).unwrap();

        let prompt = build_conflict_prompt(
            "test",
            &["conflict.rs".to_string()],
            &ConflictScan {
                has_markers: true,
                markers: vec![
                    ConflictMarker {
                        file: "conflict.rs".to_string(),
                        line: 1,
                        marker_type: MarkerType::Ours,
                    },
                    ConflictMarker {
                        file: "conflict.rs".to_string(),
                        line: 3,
                        marker_type: MarkerType::Separator,
                    },
                    ConflictMarker {
                        file: "conflict.rs".to_string(),
                        line: 5,
                        marker_type: MarkerType::Theirs,
                    },
                ],
                truncated: false,
            },
            dir.path(),
        );

        // Should contain the actual file content with markers
        assert!(prompt.contains("<<<<<<<"), "should include ours marker");
        assert!(prompt.contains("======="), "should include separator");
        assert!(prompt.contains(">>>>>>>"), "should include theirs marker");
        assert!(prompt.contains("conflict.rs"), "should include filename");
    }

    #[test]
    fn build_conflict_prompt_handles_unreadable_file() {
        let prompt = build_conflict_prompt(
            "test",
            &["nonexistent.rs".to_string()],
            &ConflictScan {
                has_markers: false,
                markers: vec![],
                truncated: false,
            },
            Path::new("/nonexistent"),
        );
        assert!(
            prompt.contains("error reading file"),
            "should show error for unreadable file"
        );
    }

    #[test]
    fn build_conflict_prompt_includes_marker_summary() {
        let prompt = build_conflict_prompt(
            "test",
            &["file.rs".to_string()],
            &ConflictScan {
                has_markers: true,
                markers: vec![ConflictMarker {
                    file: "file.rs".to_string(),
                    line: 10,
                    marker_type: MarkerType::Ours,
                }],
                truncated: false,
            },
            Path::new("/nonexistent"),
        );
        assert!(prompt.contains("Conflict Marker Summary"));
        assert!(prompt.contains("line 10"));
    }

    // ── System prompt ──────────────────────────────────────────────

    #[test]
    fn system_prompt_is_nonempty_and_instructive() {
        let sp = build_conflict_system_prompt();
        assert!(!sp.is_empty());
        assert!(sp.contains("resolving"));
        assert!(sp.contains("conflict markers"));
    }

    // ── resolve_conflict error paths ───────────────────────────────

    #[test]
    fn resolve_conflict_returns_skip_when_claude_not_found() {
        // Use a nonexistent working directory to ensure the subprocess fails
        // even if claude is somehow installed. The primary test is that
        // it doesn't panic and returns Skip gracefully.
        let config = ConflictResolutionConfig {
            enabled: true,
            model: "sonnet".to_string(),
            timeout_secs: 5,
            validation_command: None,
        };

        // This will fail because either claude is not installed,
        // or the working directory doesn't exist for git operations.
        let result = resolve_conflict(
            "test-session",
            &["file.rs".to_string()],
            &ConflictScan {
                has_markers: false,
                markers: vec![],
                truncated: false,
            },
            Path::new("/nonexistent-path-for-test"),
            &config,
        );

        assert_eq!(result.action, ConflictAction::Skip);
        assert!(!result.repo_clean);
        assert!(result.audit.is_none());
    }

    // ── run_validation_command ─────────────────────────────────────

    #[test]
    fn run_validation_command_success() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_validation_command("echo ok", dir.path(), 10);
        assert!(result.is_ok(), "expected Ok(()), got: {:?}", result);
    }

    #[test]
    fn run_validation_command_failure() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_validation_command("sh -c 'exit 1'", dir.path(), 10);
        assert!(result.is_err(), "expected Err, got Ok");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("exited with code"),
            "error should mention exit code: {msg}"
        );
    }

    #[test]
    fn run_validation_command_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_validation_command("nonexistent_binary_xyz", dir.path(), 10);
        assert!(result.is_err(), "expected Err, got Ok");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("not found"),
            "error should mention not found: {msg}"
        );
    }
}
