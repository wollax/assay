//! `SubprocessGhClient` — shells out to the system `gh` CLI binary.

use std::path::PathBuf;

use smelt_core::error::SmeltError;
use tokio::process::Command;
use tracing::{debug, warn};

use super::{GhClient, GhIssue};

// ---------------------------------------------------------------------------
// SubprocessGhClient
// ---------------------------------------------------------------------------

/// `GhClient` implementation that shells out to the system `gh` binary.
#[derive(Clone)]
pub struct SubprocessGhClient;

impl SubprocessGhClient {
    /// Resolve the path to the `gh` binary using [`which::which`].
    fn gh_binary() -> Result<PathBuf, SmeltError> {
        which::which("gh").map_err(|e| {
            SmeltError::tracker("gh_binary", format!("gh binary not found in PATH: {e}"))
        })
    }
}

impl GhClient for SubprocessGhClient {
    async fn list_issues(
        &self,
        repo: &str,
        label: &str,
        limit: u32,
    ) -> Result<Vec<GhIssue>, SmeltError> {
        let gh = Self::gh_binary()?;

        let args = [
            "issue",
            "list",
            "-R",
            repo,
            "--label",
            label,
            "--json",
            "number,title,body,url",
            "--limit",
            &limit.to_string(),
        ];

        debug!(
            repo = %repo,
            label = %label,
            limit = limit,
            cmd = ?args,
            "gh list_issues entry"
        );

        let output =
            Command::new(&gh).args(args).output().await.map_err(|e| {
                SmeltError::tracker("list_issues", format!("failed to spawn gh: {e}"))
            })?;

        let exit_code = output.status.code().unwrap_or(-1);
        if exit_code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                repo = %repo,
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "gh list_issues non-zero exit"
            );
            return Err(SmeltError::tracker(
                "list_issues",
                format!(
                    "gh issue list failed: exit_code={exit_code} stderr={}",
                    stderr.trim()
                ),
            ));
        }

        let stdout = &output.stdout;
        let issues: Vec<GhIssue> = serde_json::from_slice(stdout).map_err(|e| {
            SmeltError::tracker(
                "list_issues",
                format!("failed to parse gh JSON output: {e}"),
            )
        })?;

        Ok(issues)
    }

    async fn edit_labels(
        &self,
        repo: &str,
        number: u64,
        add_labels: &[&str],
        remove_labels: &[&str],
    ) -> Result<(), SmeltError> {
        let gh = Self::gh_binary()?;

        let number_str = number.to_string();
        let mut args = vec!["issue", "edit", "-R", repo, &number_str];

        let add_joined = add_labels.join(",");
        if !add_labels.is_empty() {
            args.push("--add-label");
            args.push(&add_joined);
        }

        let remove_joined = remove_labels.join(",");
        if !remove_labels.is_empty() {
            args.push("--remove-label");
            args.push(&remove_joined);
        }

        debug!(
            repo = %repo,
            number = number,
            add_labels = ?add_labels,
            remove_labels = ?remove_labels,
            cmd = ?args,
            "gh edit_labels entry"
        );

        let output =
            Command::new(&gh).args(args).output().await.map_err(|e| {
                SmeltError::tracker("edit_labels", format!("failed to spawn gh: {e}"))
            })?;

        let exit_code = output.status.code().unwrap_or(-1);
        if exit_code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                repo = %repo,
                number = number,
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "gh edit_labels non-zero exit"
            );
            return Err(SmeltError::tracker(
                "edit_labels",
                format!(
                    "gh issue edit failed: exit_code={exit_code} stderr={}",
                    stderr.trim()
                ),
            ));
        }

        Ok(())
    }

    async fn create_label(&self, repo: &str, name: &str) -> Result<(), SmeltError> {
        let gh = Self::gh_binary()?;

        let args = ["label", "create", "-R", repo, name, "--force"];

        debug!(
            repo = %repo,
            name = %name,
            cmd = ?args,
            "gh create_label entry"
        );

        let output =
            Command::new(&gh).args(args).output().await.map_err(|e| {
                SmeltError::tracker("create_label", format!("failed to spawn gh: {e}"))
            })?;

        let exit_code = output.status.code().unwrap_or(-1);
        if exit_code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                repo = %repo,
                name = %name,
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "gh create_label non-zero exit"
            );
            return Err(SmeltError::tracker(
                "create_label",
                format!(
                    "gh label create failed: exit_code={exit_code} stderr={}",
                    stderr.trim()
                ),
            ));
        }

        Ok(())
    }

    async fn auth_status(&self) -> Result<(), SmeltError> {
        let gh = Self::gh_binary()?;

        let args = ["auth", "status"];

        debug!(cmd = ?args, "gh auth_status entry");

        let output =
            Command::new(&gh).args(args).output().await.map_err(|e| {
                SmeltError::tracker("auth_status", format!("failed to spawn gh: {e}"))
            })?;

        let exit_code = output.status.code().unwrap_or(-1);
        if exit_code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "gh auth_status non-zero exit"
            );
            return Err(SmeltError::tracker(
                "auth_status",
                format!(
                    "gh auth status failed: exit_code={exit_code} stderr={}",
                    stderr.trim()
                ),
            ));
        }

        Ok(())
    }
}
