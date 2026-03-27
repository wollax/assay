//! [`GitCli`] — shell-out implementation of [`GitOps`].

use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::error::{Result, SmeltError};
use crate::git::{GitOps, GitWorktreeEntry, parse_porcelain};

/// Concrete [`GitOps`] implementation that shells out to the `git` binary.
#[derive(Clone)]
pub struct GitCli {
    git_binary: PathBuf,
    repo_root: PathBuf,
}

impl GitCli {
    /// Create a new `GitCli` instance.
    ///
    /// Typically constructed from the values returned by [`super::preflight()`].
    pub fn new(git_binary: PathBuf, repo_root: PathBuf) -> Self {
        Self {
            git_binary,
            repo_root,
        }
    }

    /// Run a git command in `self.repo_root` and return trimmed stdout on success.
    async fn run(&self, args: &[&str]) -> Result<String> {
        self.run_in(&self.repo_root, args).await
    }

    /// Run a git command in a specific working directory (not necessarily `self.repo_root`).
    async fn run_in(&self, work_dir: &Path, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.git_binary)
            .args(args)
            .current_dir(work_dir)
            .output()
            .await
            .map_err(|e| {
                SmeltError::io(
                    format!("running git {}", args.first().unwrap_or(&"")),
                    &self.git_binary,
                    e,
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SmeltError::GitExecution {
                operation: args.join(" "),
                message: stderr.trim().to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl GitOps for GitCli {
    async fn repo_root(&self) -> Result<PathBuf> {
        Ok(self.repo_root.clone())
    }

    async fn is_inside_work_tree(&self, path: &Path) -> Result<bool> {
        let output = Command::new(&self.git_binary)
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(path)
            .output()
            .await
            .map_err(|e| SmeltError::io("running git rev-parse --is-inside-work-tree", path, e))?;

        Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true")
    }

    async fn current_branch(&self) -> Result<String> {
        self.run(&["rev-parse", "--abbrev-ref", "HEAD"]).await
    }

    async fn head_short(&self) -> Result<String> {
        self.run(&["rev-parse", "--short", "HEAD"]).await
    }

    async fn worktree_add(&self, path: &Path, branch_name: &str, start_point: &str) -> Result<()> {
        let path_str = path.to_string_lossy();
        self.run(&["worktree", "add", "-b", branch_name, &path_str, start_point])
            .await?;
        Ok(())
    }

    async fn worktree_remove(&self, path: &Path, force: bool) -> Result<()> {
        let path_str = path.to_string_lossy();
        if force {
            self.run(&["worktree", "remove", "--force", &path_str])
                .await?;
        } else {
            self.run(&["worktree", "remove", &path_str]).await?;
        }
        Ok(())
    }

    async fn worktree_list(&self) -> Result<Vec<GitWorktreeEntry>> {
        let output = self.run(&["worktree", "list", "--porcelain"]).await?;
        Ok(parse_porcelain(&output))
    }

    async fn worktree_prune(&self) -> Result<()> {
        self.run(&["worktree", "prune"]).await?;
        Ok(())
    }

    async fn worktree_is_dirty(&self, path: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();
        let output = Command::new(&self.git_binary)
            .args(["-C", &path_str, "status", "--porcelain"])
            .output()
            .await
            .map_err(|e| SmeltError::io("running git status --porcelain", path, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SmeltError::GitExecution {
                operation: format!("-C {} status --porcelain", path_str),
                message: stderr.trim().to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    async fn branch_delete(&self, branch_name: &str, force: bool) -> Result<()> {
        let flag = if force { "-D" } else { "-d" };
        self.run(&["branch", flag, branch_name]).await?;
        Ok(())
    }

    async fn branch_is_merged(&self, branch_name: &str, base_ref: &str) -> Result<bool> {
        let output = self.run(&["branch", "--merged", base_ref]).await?;
        Ok(output.lines().any(|line| {
            let name = line.trim();
            let name = name.strip_prefix("* ").unwrap_or(name);
            name == branch_name
        }))
    }

    async fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let ref_name = format!("refs/heads/{branch_name}");
        let output = Command::new(&self.git_binary)
            .args(["rev-parse", "--verify", &ref_name])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| SmeltError::io("running git rev-parse --verify", &self.git_binary, e))?;

        Ok(output.status.success())
    }

    async fn add(&self, work_dir: &Path, paths: &[&str]) -> Result<()> {
        if paths.is_empty() {
            return Err(SmeltError::GitExecution {
                operation: "add".to_string(),
                message: "at least one file path is required".to_string(),
            });
        }
        let mut args = vec!["add"];
        args.extend(paths);
        self.run_in(work_dir, &args).await?;
        Ok(())
    }

    async fn commit(&self, work_dir: &Path, message: &str) -> Result<String> {
        self.run_in(work_dir, &["commit", "-m", message]).await?;
        // Get the short hash of the commit we just created
        let hash = self
            .run_in(work_dir, &["rev-parse", "--short", "HEAD"])
            .await?;
        Ok(hash)
    }

    async fn rev_list_count(&self, branch: &str, base: &str) -> Result<usize> {
        let range = format!("{base}..{branch}");
        let output = self.run(&["rev-list", "--count", &range]).await?;
        output
            .parse::<usize>()
            .map_err(|e| SmeltError::GitExecution {
                operation: format!("rev-list --count {range}"),
                message: format!("failed to parse count: {e}"),
            })
    }

    async fn merge_base(&self, ref_a: &str, ref_b: &str) -> Result<String> {
        self.run(&["merge-base", ref_a, ref_b]).await
    }

    async fn branch_create(&self, branch_name: &str, start_point: &str) -> Result<()> {
        self.run(&["branch", branch_name, start_point]).await?;
        Ok(())
    }

    async fn merge_squash(&self, work_dir: &Path, source_ref: &str) -> Result<()> {
        let output = Command::new(&self.git_binary)
            .args(["merge", "--squash", source_ref])
            .current_dir(work_dir)
            .output()
            .await
            .map_err(|e| SmeltError::io("running git merge --squash", work_dir, e))?;

        if output.status.success() {
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // git merge --squash writes CONFLICT markers to stdout; check stderr too as a
        // safety net because some git versions or wrappers may vary.
        if stdout.contains("CONFLICT") || stderr.contains("CONFLICT") {
            let files = self.unmerged_files(work_dir).await?;
            return Err(SmeltError::MergeConflict {
                session: String::new(),
                files,
            });
        }

        let exit_code = output.status.code().unwrap_or(-1);
        let combined = if stderr.is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        Err(SmeltError::GitExecution {
            operation: format!("merge --squash {source_ref}"),
            message: format!("exit code {exit_code}: {combined}"),
        })
    }

    async fn worktree_add_existing(&self, path: &Path, branch_name: &str) -> Result<()> {
        let path_str = path.to_string_lossy();
        self.run(&["worktree", "add", &path_str, branch_name])
            .await?;
        Ok(())
    }

    async fn unmerged_files(&self, work_dir: &Path) -> Result<Vec<String>> {
        let output = Command::new(&self.git_binary)
            .args(["diff", "--name-only", "--diff-filter=U"])
            .current_dir(work_dir)
            .output()
            .await
            .map_err(|e| {
                SmeltError::io("running git diff --name-only --diff-filter=U", work_dir, e)
            })?;

        // git diff --diff-filter=U typically exits 0 regardless of whether unmerged
        // files exist. The != 1 guard is retained defensively in case future git
        // versions use exit code 1 for this filter combination.
        let code = output.status.code().unwrap_or(-1);
        if code != 0 && code != 1 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SmeltError::GitExecution {
                operation: "diff --name-only --diff-filter=U".to_string(),
                message: format!("exit code {code}: {}", stderr.trim()),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        Ok(trimmed.lines().map(|l| l.to_string()).collect())
    }

    async fn reset_hard(&self, work_dir: &Path, target_ref: &str) -> Result<()> {
        self.run_in(work_dir, &["reset", "--hard", target_ref])
            .await?;
        Ok(())
    }

    async fn rev_parse(&self, rev: &str) -> Result<String> {
        self.run(&["rev-parse", rev]).await
    }

    async fn log_subjects(&self, range: &str) -> Result<Vec<String>> {
        let output = self.run(&["log", "--format=%s", range]).await?;
        if output.is_empty() {
            return Ok(Vec::new());
        }
        Ok(output.lines().map(|l| l.to_string()).collect())
    }

    async fn diff_name_only(&self, base_ref: &str, head_ref: &str) -> Result<Vec<String>> {
        let output = self
            .run(&["diff", "--name-only", base_ref, head_ref])
            .await?;
        if output.is_empty() {
            return Ok(Vec::new());
        }
        Ok(output.lines().map(|l| l.to_string()).collect())
    }

    async fn diff_numstat(
        &self,
        from_ref: &str,
        to_ref: &str,
    ) -> Result<Vec<(usize, usize, String)>> {
        let output = self.run(&["diff", "--numstat", from_ref, to_ref]).await?;
        if output.is_empty() {
            return Ok(Vec::new());
        }
        Ok(output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(3, '\t').collect();
                if parts.len() == 3 {
                    // Binary files produce "-" for insertions/deletions — skip them.
                    let ins: usize = parts[0].parse().ok()?;
                    let del: usize = parts[1].parse().ok()?;
                    Some((ins, del, parts[2].to_string()))
                } else {
                    None
                }
            })
            .collect())
    }

    async fn show_index_stage(&self, work_dir: &Path, stage: u8, file: &str) -> Result<String> {
        self.run_in(work_dir, &["show", &format!(":{stage}:{file}")])
            .await
    }

    async fn fetch_ref(&self, remote: &str, refspec: &str) -> Result<()> {
        self.run(&["fetch", remote, refspec]).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests;
