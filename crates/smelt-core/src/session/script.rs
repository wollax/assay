//! ScriptExecutor — runs scripted session steps in a worktree.

use std::path::{Component, PathBuf};
use std::time::Instant;

use crate::error::{Result, SmeltError};
use crate::git::GitOps;
use crate::session::manifest::{FailureMode, ScriptDef, ScriptStep};
use crate::session::types::{SessionOutcome, SessionResult};

/// Executes a scripted session definition by writing files and creating
/// git commits in a worktree.
pub struct ScriptExecutor<'a, G: GitOps> {
    git: &'a G,
    worktree_path: PathBuf,
}

impl<'a, G: GitOps> ScriptExecutor<'a, G> {
    /// Create a new `ScriptExecutor`.
    pub fn new(git: &'a G, worktree_path: PathBuf) -> Self {
        Self { git, worktree_path }
    }

    /// Validate that a file path is safe (no traversal, not absolute).
    fn validate_path(path: &str, worktree_path: &PathBuf) -> Result<PathBuf> {
        let p = std::path::Path::new(path);

        // Reject absolute paths
        if p.is_absolute() {
            return Err(SmeltError::SessionError {
                session: String::new(),
                message: format!("file path '{path}' is absolute; must be relative to worktree"),
            });
        }

        // Reject paths with .. components
        for component in p.components() {
            if matches!(component, Component::ParentDir) {
                return Err(SmeltError::SessionError {
                    session: String::new(),
                    message: format!(
                        "file path '{path}' contains '..'; must stay within worktree"
                    ),
                });
            }
        }

        // Resolve and verify the path stays under the worktree
        let resolved = worktree_path.join(p);
        if !resolved.starts_with(worktree_path) {
            return Err(SmeltError::SessionError {
                session: String::new(),
                message: format!("file path '{path}' resolves outside worktree"),
            });
        }

        Ok(resolved)
    }

    /// Execute a scripted session definition.
    ///
    /// Iterates over script steps, writing files and creating git commits.
    /// Respects `exit_after` for early termination and `simulate_failure`
    /// for failure mode simulation.
    pub async fn execute(&self, session_name: &str, script: &ScriptDef) -> Result<SessionResult> {
        let start_time = Instant::now();
        let max_steps = script
            .exit_after
            .unwrap_or(script.steps.len())
            .min(script.steps.len());

        let mut steps_completed: usize = 0;

        for step in &script.steps[..max_steps] {
            match step {
                ScriptStep::Commit { message, files } => {
                    // Handle Partial failure: only write first half of files
                    let effective_files =
                        if matches!(script.simulate_failure, Some(FailureMode::Partial)) {
                            let count = (files.len() / 2).max(1);
                            &files[..count]
                        } else {
                            files.as_slice()
                        };

                    // Write files to worktree
                    for file_change in effective_files {
                        let file_path =
                            Self::validate_path(&file_change.path, &self.worktree_path)?;

                        // Create parent directories
                        if let Some(parent) = file_path.parent() {
                            std::fs::create_dir_all(parent).map_err(|e| {
                                SmeltError::io("creating parent directories", parent, e)
                            })?;
                        }

                        // Determine content
                        let content = if let Some(ref c) = file_change.content {
                            c.clone()
                        } else if let Some(ref content_file) = file_change.content_file {
                            std::fs::read_to_string(content_file).map_err(|e| {
                                SmeltError::io(
                                    "reading content_file",
                                    PathBuf::from(content_file),
                                    e,
                                )
                            })?
                        } else {
                            String::new()
                        };

                        std::fs::write(&file_path, &content)
                            .map_err(|e| SmeltError::io("writing file to worktree", &file_path, e))?;
                    }

                    // Stage files
                    let file_paths: Vec<&str> =
                        effective_files.iter().map(|f| f.path.as_str()).collect();
                    self.git.add(&self.worktree_path, &file_paths).await?;

                    // Commit
                    self.git.commit(&self.worktree_path, message).await?;
                    steps_completed += 1;

                    // If Partial, fail after the first commit step
                    if matches!(script.simulate_failure, Some(FailureMode::Partial)) {
                        return Ok(SessionResult {
                            session_name: session_name.to_string(),
                            outcome: SessionOutcome::Failed,
                            steps_completed,
                            failure_reason: Some("simulated partial failure".to_string()),
                            has_commits: steps_completed > 0,
                            duration: start_time.elapsed(),
                        });
                    }
                }
            }
        }

        // Handle failure modes after completing max_steps
        if let Some(ref failure_mode) = script.simulate_failure {
            let (outcome, reason) = match failure_mode {
                FailureMode::Crash => {
                    (SessionOutcome::Failed, "simulated crash".to_string())
                }
                FailureMode::Hang => (
                    SessionOutcome::Failed,
                    "hang simulation not implemented".to_string(),
                ),
                FailureMode::Partial => {
                    // Already handled above; shouldn't reach here
                    (SessionOutcome::Failed, "simulated partial failure".to_string())
                }
            };

            return Ok(SessionResult {
                session_name: session_name.to_string(),
                outcome,
                steps_completed,
                failure_reason: Some(reason),
                has_commits: steps_completed > 0,
                duration: start_time.elapsed(),
            });
        }

        Ok(SessionResult {
            session_name: session_name.to_string(),
            outcome: SessionOutcome::Completed,
            steps_completed,
            failure_reason: None,
            has_commits: steps_completed > 0,
            duration: start_time.elapsed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitCli;
    use crate::session::manifest::FileChange;
    use std::process::Command;

    /// Create a temporary git repo with an initial commit and a worktree,
    /// returning (temp_dir, GitCli, worktree_path, default_branch).
    async fn setup_test_repo_with_worktree() -> (tempfile::TempDir, GitCli, PathBuf, String) {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let repo_path = tmp.path().join("test-repo");
        std::fs::create_dir(&repo_path).expect("create repo dir");

        let git = which::which("git").expect("git on PATH");

        Command::new(&git)
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .expect("git init");

        for args in [
            &["config", "user.email", "test@example.com"][..],
            &["config", "user.name", "Test"][..],
        ] {
            Command::new(&git)
                .args(args)
                .current_dir(&repo_path)
                .output()
                .expect("git config");
        }

        std::fs::write(repo_path.join("README.md"), "# test\n").unwrap();
        Command::new(&git)
            .args(["add", "README.md"])
            .current_dir(&repo_path)
            .output()
            .expect("git add");
        Command::new(&git)
            .args(["commit", "-m", "initial"])
            .current_dir(&repo_path)
            .output()
            .expect("git commit");

        let cli = GitCli::new(git.clone(), repo_path.clone());

        // Get default branch name
        let default_branch = cli.current_branch().await.expect("current_branch");

        // Create worktree
        let wt_path = tmp.path().join("test-worktree");
        cli.worktree_add(&wt_path, "smelt/test-session", "HEAD")
            .await
            .expect("worktree_add");

        (tmp, cli, wt_path, default_branch)
    }

    fn make_script(steps: Vec<ScriptStep>) -> ScriptDef {
        ScriptDef {
            backend: "scripted".to_string(),
            exit_after: None,
            simulate_failure: None,
            steps,
        }
    }

    fn commit_step(message: &str, files: Vec<(&str, &str)>) -> ScriptStep {
        ScriptStep::Commit {
            message: message.to_string(),
            files: files
                .into_iter()
                .map(|(path, content)| FileChange {
                    path: path.to_string(),
                    content: Some(content.to_string()),
                    content_file: None,
                })
                .collect(),
        }
    }

    #[tokio::test]
    async fn execute_two_step_script_creates_two_commits() {
        let (_tmp, cli, wt_path, default_branch) = setup_test_repo_with_worktree().await;

        let script = make_script(vec![
            commit_step("first commit", vec![("a.txt", "aaa\n")]),
            commit_step("second commit", vec![("b.txt", "bbb\n")]),
        ]);

        let executor = ScriptExecutor::new(&cli, wt_path);
        let result = executor.execute("test-session", &script).await.expect("execute");

        assert_eq!(result.outcome, SessionOutcome::Completed);
        assert_eq!(result.steps_completed, 2);
        assert!(result.has_commits);
        assert!(result.failure_reason.is_none());

        // Verify 2 commits on the branch (above the base)
        let count = cli
            .rev_list_count("smelt/test-session", &default_branch)
            .await
            .expect("rev_list_count");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn exit_after_truncates_execution() {
        let (_tmp, cli, wt_path, default_branch) = setup_test_repo_with_worktree().await;

        let mut script = make_script(vec![
            commit_step("first", vec![("a.txt", "a\n")]),
            commit_step("second", vec![("b.txt", "b\n")]),
            commit_step("third", vec![("c.txt", "c\n")]),
        ]);
        script.exit_after = Some(1);

        let executor = ScriptExecutor::new(&cli, wt_path);
        let result = executor.execute("test-session", &script).await.expect("execute");

        assert_eq!(result.outcome, SessionOutcome::Completed);
        assert_eq!(result.steps_completed, 1);

        let count = cli
            .rev_list_count("smelt/test-session", &default_branch)
            .await
            .expect("rev_list_count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn simulate_failure_crash_returns_failed() {
        let (_tmp, cli, wt_path, default_branch) = setup_test_repo_with_worktree().await;

        let mut script = make_script(vec![
            commit_step("first", vec![("a.txt", "a\n")]),
            commit_step("second", vec![("b.txt", "b\n")]),
        ]);
        script.exit_after = Some(1);
        script.simulate_failure = Some(FailureMode::Crash);

        let executor = ScriptExecutor::new(&cli, wt_path);
        let result = executor.execute("test-session", &script).await.expect("execute");

        assert_eq!(result.outcome, SessionOutcome::Failed);
        assert_eq!(result.steps_completed, 1);
        assert!(result.has_commits);
        assert_eq!(result.failure_reason.as_deref(), Some("simulated crash"));

        // Still created 1 commit
        let count = cli
            .rev_list_count("smelt/test-session", &default_branch)
            .await
            .expect("rev_list_count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn files_are_written_to_worktree() {
        let (_tmp, cli, wt_path, _default_branch) = setup_test_repo_with_worktree().await;

        let script = make_script(vec![commit_step(
            "write files",
            vec![("hello.txt", "hello world\n")],
        )]);

        let executor = ScriptExecutor::new(&cli, wt_path.clone());
        executor.execute("test-session", &script).await.expect("execute");

        let content = std::fs::read_to_string(wt_path.join("hello.txt")).expect("read file");
        assert_eq!(content, "hello world\n");
    }

    #[tokio::test]
    async fn simulate_failure_partial_writes_half_files_then_fails() {
        let (_tmp, cli, wt_path, default_branch) = setup_test_repo_with_worktree().await;

        let mut script = make_script(vec![ScriptStep::Commit {
            message: "partial commit".to_string(),
            files: vec![
                FileChange {
                    path: "a.txt".to_string(),
                    content: Some("aaa\n".to_string()),
                    content_file: None,
                },
                FileChange {
                    path: "b.txt".to_string(),
                    content: Some("bbb\n".to_string()),
                    content_file: None,
                },
                FileChange {
                    path: "c.txt".to_string(),
                    content: Some("ccc\n".to_string()),
                    content_file: None,
                },
                FileChange {
                    path: "d.txt".to_string(),
                    content: Some("ddd\n".to_string()),
                    content_file: None,
                },
            ],
        }]);
        script.simulate_failure = Some(FailureMode::Partial);

        let executor = ScriptExecutor::new(&cli, wt_path);
        let result = executor
            .execute("test-session", &script)
            .await
            .expect("execute");

        assert_eq!(result.outcome, SessionOutcome::Failed);
        assert_eq!(result.steps_completed, 1);
        assert!(result.has_commits);
        assert_eq!(
            result.failure_reason.as_deref(),
            Some("simulated partial failure")
        );

        // Should have created exactly 1 commit (partial, then fail)
        let count = cli
            .rev_list_count("smelt/test-session", &default_branch)
            .await
            .expect("rev_list_count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn path_traversal_absolute_rejected() {
        let (_tmp, cli, wt_path, _) = setup_test_repo_with_worktree().await;

        let script = make_script(vec![ScriptStep::Commit {
            message: "bad path".to_string(),
            files: vec![FileChange {
                path: "/etc/passwd".to_string(),
                content: Some("pwned".to_string()),
                content_file: None,
            }],
        }]);

        let executor = ScriptExecutor::new(&cli, wt_path);
        let err = executor
            .execute("test-session", &script)
            .await
            .expect_err("should reject absolute path");
        assert!(err.to_string().contains("absolute"), "got: {err}");
    }

    #[tokio::test]
    async fn path_traversal_dotdot_rejected() {
        let (_tmp, cli, wt_path, _) = setup_test_repo_with_worktree().await;

        let script = make_script(vec![ScriptStep::Commit {
            message: "bad path".to_string(),
            files: vec![FileChange {
                path: "../../.ssh/authorized_keys".to_string(),
                content: Some("pwned".to_string()),
                content_file: None,
            }],
        }]);

        let executor = ScriptExecutor::new(&cli, wt_path);
        let err = executor
            .execute("test-session", &script)
            .await
            .expect_err("should reject .. path");
        assert!(err.to_string().contains(".."), "got: {err}");
    }

    #[tokio::test]
    async fn content_file_reads_from_disk() {
        let (_tmp, cli, wt_path, _) = setup_test_repo_with_worktree().await;

        // Write a content file in the worktree
        let content_path = wt_path.join("template.txt");
        std::fs::write(&content_path, "content from file\n").unwrap();

        let script = make_script(vec![ScriptStep::Commit {
            message: "from file".to_string(),
            files: vec![FileChange {
                path: "output.txt".to_string(),
                content: None,
                content_file: Some(content_path.to_string_lossy().to_string()),
            }],
        }]);

        let executor = ScriptExecutor::new(&cli, wt_path.clone());
        let result = executor
            .execute("test-session", &script)
            .await
            .expect("execute");

        assert_eq!(result.outcome, SessionOutcome::Completed);
        let content = std::fs::read_to_string(wt_path.join("output.txt")).expect("read output");
        assert_eq!(content, "content from file\n");
    }

    #[tokio::test]
    async fn parent_directories_are_created() {
        let (_tmp, cli, wt_path, _default_branch) = setup_test_repo_with_worktree().await;

        let script = make_script(vec![commit_step(
            "nested file",
            vec![("src/auth/login.rs", "pub fn login() {}\n")],
        )]);

        let executor = ScriptExecutor::new(&cli, wt_path.clone());
        executor.execute("test-session", &script).await.expect("execute");

        assert!(wt_path.join("src/auth/login.rs").exists());
        let content =
            std::fs::read_to_string(wt_path.join("src/auth/login.rs")).expect("read file");
        assert_eq!(content, "pub fn login() {}\n");
    }
}
