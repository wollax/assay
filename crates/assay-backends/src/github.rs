//! GitHubBackend — persists orchestrator state as GitHub issues and comments.
//!
//! `GhRunner` wraps `std::process::Command` for synchronous `gh` CLI
//! calls. `GitHubBackend` implements [`StateBackend`] by creating an
//! issue per `run_dir` on the first `push_session_event`, appending
//! comments on subsequent calls, and reading the latest comment body
//! for `read_run_state`.
//!
//! The issue number is cached in a `.github-issue-number` file inside `run_dir`.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use assay_core::{AssayError, CapabilitySet, StateBackend};
use assay_types::{OrchestratorStatus, TeamCheckpoint};
use serde_json::Value;

// ---------------------------------------------------------------------------
// GhRunner — low-level `gh` CLI wrapper
// ---------------------------------------------------------------------------

/// Low-level wrapper around the `gh` CLI for GitHub issue operations.
///
/// All commands use `Command::arg()` chaining (no shell string interpolation)
/// and pass `--repo` explicitly for every invocation.
struct GhRunner {
    repo: String,
}

impl GhRunner {
    /// Build a structured [`AssayError`] from a failed `gh` command output.
    ///
    /// Logs a warning with the repo, exit code, and stderr trim, then constructs
    /// and **returns the bare error value** (not a `Result`). Callers wrap it:
    /// `return Err(self.gh_error("gh issue create", &output, None));`
    fn gh_error(&self, operation: &str, output: &Output, issue_number: Option<u64>) -> AssayError {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        tracing::warn!(
            repo = %self.repo,
            issue_number,
            exit_code = output.status.code(),
            stderr = %stderr,
            "{operation} failed"
        );
        AssayError::io(
            format!("{operation} failed: {stderr}"),
            "gh",
            std::io::Error::other(stderr.to_string()),
        )
    }

    /// Create a GitHub issue and return its number.
    ///
    /// Runs `gh issue create --repo <repo> --title <title> --body-file - [--label <label>]`
    /// with the body piped via stdin. Parses the issue number from the URL
    /// printed to stdout.
    fn create_issue(
        &self,
        title: &str,
        body: &str,
        label: Option<&str>,
    ) -> assay_core::Result<u64> {
        let mut cmd = Command::new("gh");
        cmd.arg("issue")
            .arg("create")
            .arg("--repo")
            .arg(&self.repo)
            .arg("--title")
            .arg(title)
            .arg("--body-file")
            .arg("-");

        if let Some(lbl) = label {
            cmd.arg("--label").arg(lbl);
        }

        tracing::debug!(repo = %self.repo, title, "constructing gh issue create command");

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| AssayError::io("spawning gh issue create", "gh", e))?;

        // Write body to stdin.
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(body.as_bytes())
                .map_err(|e| AssayError::io("writing body to gh issue create stdin", "gh", e))?;
            // stdin is dropped here, closing the pipe.
        }

        let output = child
            .wait_with_output()
            .map_err(|e| AssayError::io("waiting for gh issue create", "gh", e))?;

        if !output.status.success() {
            return Err(self.gh_error("gh issue create", &output, None));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issue_number = stdout
            .trim()
            .rsplit('/')
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| {
                AssayError::io(
                    format!("parsing issue number from gh output: {}", stdout.trim()),
                    "gh",
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unexpected gh issue create output: {}", stdout.trim()),
                    ),
                )
            })?;

        tracing::info!(issue_number, repo = %self.repo, "created GitHub issue");
        Ok(issue_number)
    }

    /// Create a comment on an existing issue.
    ///
    /// Runs `gh issue comment <number> --repo <repo> --body-file -`
    /// with the body piped via stdin.
    fn create_comment(&self, issue_number: u64, body: &str) -> assay_core::Result<()> {
        let mut cmd = Command::new("gh");
        cmd.arg("issue")
            .arg("comment")
            .arg(issue_number.to_string())
            .arg("--repo")
            .arg(&self.repo)
            .arg("--body-file")
            .arg("-");

        tracing::debug!(repo = %self.repo, issue_number, "constructing gh issue comment command");

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| AssayError::io("spawning gh issue comment", "gh", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(body.as_bytes())
                .map_err(|e| AssayError::io("writing body to gh issue comment stdin", "gh", e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| AssayError::io("waiting for gh issue comment", "gh", e))?;

        if !output.status.success() {
            return Err(self.gh_error("gh issue comment", &output, Some(issue_number)));
        }

        tracing::info!(issue_number, repo = %self.repo, "created GitHub comment");
        Ok(())
    }

    /// Fetch issue JSON including body and comments.
    ///
    /// Runs `gh issue view <number> --repo <repo> --json body,comments`
    /// and parses stdout as JSON.
    fn get_issue_json(&self, issue_number: u64) -> assay_core::Result<Value> {
        let mut cmd = Command::new("gh");
        cmd.arg("issue")
            .arg("view")
            .arg(issue_number.to_string())
            .arg("--repo")
            .arg(&self.repo)
            .arg("--json")
            .arg("body,comments");

        tracing::debug!(repo = %self.repo, issue_number, "constructing gh issue view command");

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| AssayError::io("running gh issue view", "gh", e))?;

        if !output.status.success() {
            return Err(self.gh_error("gh issue view", &output, Some(issue_number)));
        }

        let json: Value = serde_json::from_slice(&output.stdout).map_err(|e| {
            AssayError::io(
                "parsing gh issue view JSON output",
                "gh",
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
            )
        })?;

        Ok(json)
    }
}

// ---------------------------------------------------------------------------
// GitHubBackend
// ---------------------------------------------------------------------------

/// Remote backend that persists orchestrator state as GitHub issues/comments.
///
/// Each `run_dir` maps to one GitHub issue. The issue number is cached locally
/// in `run_dir/.github-issue-number`. On the first `push_session_event` for a
/// `run_dir`, an issue is created. Subsequent calls append a comment with the
/// serialized `OrchestratorStatus` JSON.
///
/// `read_run_state` fetches the issue via `gh issue view --json body,comments`,
/// extracts the latest comment body (or falls back to the issue body if no
/// comments), and deserializes it as `OrchestratorStatus`.
pub struct GitHubBackend {
    runner: GhRunner,
    label: Option<String>,
}

impl std::fmt::Debug for GitHubBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubBackend")
            .field("repo", &self.runner.repo)
            .field("label", &self.label)
            .finish()
    }
}

impl GitHubBackend {
    /// Construct a `GitHubBackend` targeting the given repository.
    ///
    /// `repo` must be in `owner/repo` format. If `repo` is empty or contains no
    /// `/`, a `tracing::warn!` is emitted at construction time; the constructor
    /// remains infallible and the backend is still created (D177).
    /// `label` is an optional label applied to created issues.
    pub fn new(repo: String, label: Option<String>) -> Self {
        if repo.is_empty() || !repo.contains('/') {
            tracing::warn!(repo = %repo, "malformed GitHub repo — expected 'owner/repo' format");
        }
        Self {
            runner: GhRunner { repo },
            label,
        }
    }

    /// Read the cached issue number from `.github-issue-number` in `run_dir`.
    ///
    /// Returns `Err` if the file contains `0` (treated as file corruption).
    fn read_issue_number(run_dir: &Path) -> assay_core::Result<Option<u64>> {
        let path = run_dir.join(".github-issue-number");
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                let trimmed = contents.trim();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    let number = trimmed.parse::<u64>().map_err(|_| {
                        AssayError::io(
                            "parsing .github-issue-number",
                            &path,
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("invalid issue number: {trimmed}"),
                            ),
                        )
                    })?;
                    if number == 0 {
                        return Err(AssayError::io(
                            "invalid issue number: 0 (possible file corruption)",
                            &path,
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "issue number must be non-zero",
                            ),
                        ));
                    }
                    Ok(Some(number))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AssayError::io("reading .github-issue-number", &path, e)),
        }
    }

    /// Write the issue number to `.github-issue-number` in `run_dir`.
    fn write_issue_number(run_dir: &Path, number: u64) -> assay_core::Result<()> {
        let path = run_dir.join(".github-issue-number");
        std::fs::write(&path, number.to_string())
            .map_err(|e| AssayError::io("writing .github-issue-number", &path, e))
    }
}

impl StateBackend for GitHubBackend {
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet::none()
    }

    fn push_session_event(
        &self,
        run_dir: &Path,
        status: &OrchestratorStatus,
    ) -> assay_core::Result<()> {
        let status_json = serde_json::to_string_pretty(status)
            .map_err(|e| AssayError::json("serializing OrchestratorStatus", run_dir, e))?;

        match Self::read_issue_number(run_dir)? {
            Some(issue_number) => {
                // Subsequent call — append as comment.
                self.runner.create_comment(issue_number, &status_json)?;
            }
            None => {
                // First call — create issue, cache number.
                let title = run_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "assay-run".to_string());

                let issue_number =
                    self.runner
                        .create_issue(&title, &status_json, self.label.as_deref())?;
                Self::write_issue_number(run_dir, issue_number)?;
            }
        }

        Ok(())
    }

    fn read_run_state(&self, run_dir: &Path) -> assay_core::Result<Option<OrchestratorStatus>> {
        let issue_number = match Self::read_issue_number(run_dir)? {
            Some(n) => n,
            None => return Ok(None),
        };

        let json = self.runner.get_issue_json(issue_number)?;

        // Extract the latest comment body, or fall back to the issue body.
        let body_str = json
            .get("comments")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.last())
            .and_then(|c| c.get("body"))
            .and_then(|b| b.as_str())
            .or_else(|| json.get("body").and_then(|b| b.as_str()));

        let body_str = match body_str {
            Some(b) => b,
            None => return Ok(None),
        };

        let status: OrchestratorStatus = serde_json::from_str(body_str).map_err(|e| {
            AssayError::json(
                "deserializing OrchestratorStatus from GitHub comment",
                run_dir,
                e,
            )
        })?;

        Ok(Some(status))
    }

    fn send_message(
        &self,
        _inbox_path: &Path,
        _name: &str,
        _contents: &[u8],
    ) -> assay_core::Result<()> {
        Err(AssayError::io(
            "send_message not supported by GitHubBackend",
            "GitHubBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "GitHubBackend does not support messaging",
            ),
        ))
    }

    fn poll_inbox(&self, _inbox_path: &Path) -> assay_core::Result<Vec<(String, Vec<u8>)>> {
        Err(AssayError::io(
            "poll_inbox not supported by GitHubBackend",
            "GitHubBackend",
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "GitHubBackend does not support messaging",
            ),
        ))
    }

    fn annotate_run(&self, _run_dir: &Path, _manifest_path: &str) -> assay_core::Result<()> {
        // Silent no-op — capabilities().supports_annotations is false.
        Ok(())
    }

    fn save_checkpoint_summary(
        &self,
        _assay_dir: &Path,
        _checkpoint: &TeamCheckpoint,
    ) -> assay_core::Result<()> {
        // Silent no-op — capabilities().supports_checkpoints is false.
        Ok(())
    }
}
