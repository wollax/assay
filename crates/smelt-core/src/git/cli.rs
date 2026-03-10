//! [`GitCli`] — shell-out implementation of [`GitOps`].

use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::error::{Result, SmeltError};
use crate::git::GitOps;
use crate::worktree::{GitWorktreeEntry, parse_porcelain};

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

    /// Run a git command and return trimmed stdout on success.
    async fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.git_binary)
            .args(args)
            .current_dir(&self.repo_root)
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

    async fn worktree_add(
        &self,
        path: &Path,
        branch_name: &str,
        start_point: &str,
    ) -> Result<()> {
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
            .map_err(|e| {
                SmeltError::io("running git status --porcelain", path, e)
            })?;

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

    async fn branch_is_merged(
        &self,
        branch_name: &str,
        base_ref: &str,
    ) -> Result<bool> {
        let output = self.run(&["branch", "--merged", base_ref]).await?;
        Ok(output
            .lines()
            .any(|line| line.trim().trim_start_matches("* ") == branch_name))
    }

    async fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let ref_name = format!("refs/heads/{branch_name}");
        let output = Command::new(&self.git_binary)
            .args(["rev-parse", "--verify", &ref_name])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| {
                SmeltError::io("running git rev-parse --verify", &self.git_binary, e)
            })?;

        Ok(output.status.success())
    }

    async fn add(&self, work_dir: &Path, paths: &[&str]) -> Result<()> {
        assert!(!paths.is_empty(), "add() requires explicit file paths");
        let mut args = vec!["add"];
        args.extend(paths);
        self.run_in(work_dir, &args).await?;
        Ok(())
    }

    async fn commit(&self, work_dir: &Path, message: &str) -> Result<String> {
        self.run_in(work_dir, &["commit", "-m", message]).await?;
        // Get the short hash of the commit we just created
        let hash = self.run_in(work_dir, &["rev-parse", "--short", "HEAD"]).await?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a temporary git repo with an initial commit, returning (temp_dir, GitCli).
    fn setup_test_repo() -> (tempfile::TempDir, GitCli) {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let git = which::which("git").expect("git on PATH");

        // git init
        let status = std::process::Command::new(&git)
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .expect("git init");
        assert!(status.status.success(), "git init failed");

        // Configure user for commits
        for args in [
            &["config", "user.email", "test@example.com"][..],
            &["config", "user.name", "Test"][..],
        ] {
            std::process::Command::new(&git)
                .args(args)
                .current_dir(tmp.path())
                .output()
                .expect("git config");
        }

        // Create initial commit
        std::fs::write(tmp.path().join("README.md"), "# test\n").unwrap();
        std::process::Command::new(&git)
            .args(["add", "README.md"])
            .current_dir(tmp.path())
            .output()
            .expect("git add");
        std::process::Command::new(&git)
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .expect("git commit");

        let cli = GitCli::new(git, tmp.path().to_path_buf());
        (tmp, cli)
    }

    #[tokio::test]
    async fn test_repo_root() {
        let (tmp, cli) = setup_test_repo();
        let root = cli.repo_root().await.expect("repo_root");
        // Canonicalize both to handle macOS /private/var symlink
        assert_eq!(
            root.canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap(),
        );
    }

    #[tokio::test]
    async fn test_current_branch() {
        let (_tmp, cli) = setup_test_repo();
        let branch = cli.current_branch().await.expect("current_branch");
        // Default branch name may be "main" or "master" depending on git config
        assert!(
            branch == "main" || branch == "master",
            "expected main or master, got: {branch}",
        );
    }

    #[tokio::test]
    async fn test_head_short() {
        let (_tmp, cli) = setup_test_repo();
        let hash = cli.head_short().await.expect("head_short");
        // Short hash is typically 7-12 hex characters
        assert!(
            hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()),
            "expected short hex hash, got: {hash}",
        );
    }

    #[tokio::test]
    async fn test_is_inside_work_tree() {
        let (tmp, cli) = setup_test_repo();
        assert!(cli.is_inside_work_tree(tmp.path()).await.expect("check"));
    }

    #[tokio::test]
    async fn test_worktree_add_and_list() {
        let (tmp, cli) = setup_test_repo();
        let wt_path = tmp.path().parent().unwrap().join("smelt-test-wt-add");

        cli.worktree_add(&wt_path, "test-branch", "HEAD")
            .await
            .expect("worktree_add");

        let entries = cli.worktree_list().await.expect("worktree_list");
        assert!(entries.len() >= 2, "should have main + new worktree");

        let wt_entry = entries
            .iter()
            .find(|e| e.branch.as_deref() == Some("test-branch"));
        assert!(wt_entry.is_some(), "should find the new worktree entry");

        // Cleanup
        cli.worktree_remove(&wt_path, false)
            .await
            .expect("worktree_remove");
        let _ = std::fs::remove_dir_all(&wt_path);
    }

    #[tokio::test]
    async fn test_worktree_remove() {
        let (tmp, cli) = setup_test_repo();
        let wt_path = tmp.path().parent().unwrap().join("smelt-test-wt-remove");

        cli.worktree_add(&wt_path, "remove-branch", "HEAD")
            .await
            .expect("worktree_add");

        cli.worktree_remove(&wt_path, false)
            .await
            .expect("worktree_remove");

        let entries = cli.worktree_list().await.expect("worktree_list");
        let found = entries
            .iter()
            .any(|e| e.branch.as_deref() == Some("remove-branch"));
        assert!(!found, "worktree should be gone after remove");

        let _ = std::fs::remove_dir_all(&wt_path);
    }

    #[tokio::test]
    async fn test_branch_exists() {
        let (_tmp, cli) = setup_test_repo();

        let default_branch = cli.current_branch().await.expect("current_branch");
        assert!(
            cli.branch_exists(&default_branch).await.expect("exists"),
            "default branch should exist"
        );

        assert!(
            !cli.branch_exists("nonexistent-branch-xyz")
                .await
                .expect("not exists"),
            "nonexistent branch should not exist"
        );
    }

    #[tokio::test]
    async fn test_branch_delete() {
        let (tmp, cli) = setup_test_repo();
        let git = which::which("git").expect("git on PATH");

        // Create a branch to delete
        std::process::Command::new(&git)
            .args(["branch", "delete-me"])
            .current_dir(tmp.path())
            .output()
            .expect("create branch");

        assert!(cli.branch_exists("delete-me").await.expect("exists"));

        cli.branch_delete("delete-me", false)
            .await
            .expect("branch_delete");

        assert!(!cli.branch_exists("delete-me").await.expect("not exists"));
    }

    #[tokio::test]
    async fn test_worktree_is_dirty() {
        let (tmp, cli) = setup_test_repo();
        let wt_path = tmp.path().parent().unwrap().join("smelt-test-wt-dirty");

        cli.worktree_add(&wt_path, "dirty-branch", "HEAD")
            .await
            .expect("worktree_add");

        // Clean worktree
        let dirty = cli
            .worktree_is_dirty(&wt_path)
            .await
            .expect("is_dirty clean");
        assert!(!dirty, "freshly created worktree should be clean");

        // Create untracked file to make it dirty
        std::fs::write(wt_path.join("untracked.txt"), "dirty\n").expect("write file");

        let dirty = cli
            .worktree_is_dirty(&wt_path)
            .await
            .expect("is_dirty dirty");
        assert!(dirty, "worktree with untracked file should be dirty");

        // Cleanup
        cli.worktree_remove(&wt_path, true)
            .await
            .expect("worktree_remove");
        let _ = std::fs::remove_dir_all(&wt_path);
    }

    #[tokio::test]
    async fn test_add_and_commit() {
        let (tmp, cli) = setup_test_repo();
        let default_branch = cli.current_branch().await.expect("current_branch");

        // Create a file and commit it via the new methods
        std::fs::write(tmp.path().join("new_file.txt"), "hello\n").unwrap();
        cli.add(tmp.path(), &["new_file.txt"])
            .await
            .expect("add");
        let hash = cli
            .commit(tmp.path(), "add new file")
            .await
            .expect("commit");

        // Verify hash is valid hex
        assert!(
            hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()),
            "expected short hex hash, got: {hash}"
        );

        // Verify rev_list_count sees the new commit
        // We need to get the initial commit hash first
        let count = cli
            .rev_list_count(&default_branch, &format!("{default_branch}~1"))
            .await
            .expect("rev_list_count");
        assert!(count >= 1, "should have at least 1 commit ahead");
    }

    #[tokio::test]
    async fn test_commit_returns_valid_hash() {
        let (tmp, cli) = setup_test_repo();

        std::fs::write(tmp.path().join("hash_test.txt"), "test\n").unwrap();
        cli.add(tmp.path(), &["hash_test.txt"]).await.expect("add");
        let hash = cli
            .commit(tmp.path(), "test hash")
            .await
            .expect("commit");

        assert!(
            hash.len() >= 7 && hash.len() <= 12,
            "short hash should be 7-12 chars, got: {hash}"
        );
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash should be hex, got: {hash}"
        );
    }

    #[tokio::test]
    async fn test_rev_list_count() {
        let (tmp, cli) = setup_test_repo();
        let default_branch = cli.current_branch().await.expect("current_branch");
        let git = which::which("git").expect("git on PATH");

        // Create a feature branch at the same point
        std::process::Command::new(&git)
            .args(["branch", "count-test"])
            .current_dir(tmp.path())
            .output()
            .expect("create branch");

        // Same point: 0 commits ahead
        let count = cli
            .rev_list_count("count-test", &default_branch)
            .await
            .expect("rev_list_count");
        assert_eq!(count, 0, "branches at same point should have 0 diff");

        // Add 2 commits to count-test branch
        std::process::Command::new(&git)
            .args(["checkout", "count-test"])
            .current_dir(tmp.path())
            .output()
            .expect("checkout");

        for i in 0..2 {
            std::fs::write(
                tmp.path().join(format!("count_{i}.txt")),
                format!("content {i}\n"),
            )
            .unwrap();
            std::process::Command::new(&git)
                .args(["add", &format!("count_{i}.txt")])
                .current_dir(tmp.path())
                .output()
                .expect("git add");
            std::process::Command::new(&git)
                .args(["commit", "-m", &format!("commit {i}")])
                .current_dir(tmp.path())
                .output()
                .expect("git commit");
        }

        // Go back to default branch
        std::process::Command::new(&git)
            .args(["checkout", &default_branch])
            .current_dir(tmp.path())
            .output()
            .expect("checkout default");

        let count = cli
            .rev_list_count("count-test", &default_branch)
            .await
            .expect("rev_list_count");
        assert_eq!(count, 2, "count-test should be 2 commits ahead");
    }

    #[tokio::test]
    async fn test_add_specific_paths() {
        let (tmp, cli) = setup_test_repo();

        // Create two files but only stage one
        std::fs::write(tmp.path().join("staged.txt"), "staged\n").unwrap();
        std::fs::write(tmp.path().join("not_staged.txt"), "not staged\n").unwrap();

        cli.add(tmp.path(), &["staged.txt"]).await.expect("add");
        cli.commit(tmp.path(), "only staged file")
            .await
            .expect("commit");

        // The not_staged.txt should still be untracked
        let dirty = cli
            .worktree_is_dirty(tmp.path())
            .await
            .expect("is_dirty");
        assert!(dirty, "not_staged.txt should still be untracked");
    }

    #[tokio::test]
    async fn test_add_and_commit_in_worktree() {
        let (tmp, cli) = setup_test_repo();
        let wt_path = tmp.path().parent().unwrap().join("smelt-test-wt-commit");
        let default_branch = cli.current_branch().await.expect("current_branch");

        cli.worktree_add(&wt_path, "wt-commit-branch", "HEAD")
            .await
            .expect("worktree_add");

        // Write, stage, and commit in the worktree
        std::fs::write(wt_path.join("wt_file.txt"), "worktree content\n").unwrap();
        cli.add(&wt_path, &["wt_file.txt"]).await.expect("add in wt");
        let hash = cli
            .commit(&wt_path, "commit in worktree")
            .await
            .expect("commit in wt");

        assert!(
            hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()),
            "expected valid hash from worktree commit, got: {hash}"
        );

        // Verify the commit is on the worktree branch, not on default
        let count = cli
            .rev_list_count("wt-commit-branch", &default_branch)
            .await
            .expect("rev_list_count");
        assert_eq!(count, 1, "worktree branch should be 1 commit ahead");

        // Cleanup
        cli.worktree_remove(&wt_path, false)
            .await
            .expect("worktree_remove");
        let _ = std::fs::remove_dir_all(&wt_path);
    }

    #[tokio::test]
    async fn test_branch_is_merged() {
        let (tmp, cli) = setup_test_repo();
        let git = which::which("git").expect("git on PATH");
        let default_branch = cli.current_branch().await.expect("current_branch");

        // Create and checkout a feature branch, make a commit, merge it back
        std::process::Command::new(&git)
            .args(["checkout", "-b", "merged-branch"])
            .current_dir(tmp.path())
            .output()
            .expect("checkout branch");

        std::fs::write(tmp.path().join("feature.txt"), "feature\n").unwrap();
        std::process::Command::new(&git)
            .args(["add", "feature.txt"])
            .current_dir(tmp.path())
            .output()
            .expect("git add");
        std::process::Command::new(&git)
            .args(["commit", "-m", "feature commit"])
            .current_dir(tmp.path())
            .output()
            .expect("git commit");

        // Go back to default branch and merge
        std::process::Command::new(&git)
            .args(["checkout", &default_branch])
            .current_dir(tmp.path())
            .output()
            .expect("checkout default");
        std::process::Command::new(&git)
            .args(["merge", "merged-branch"])
            .current_dir(tmp.path())
            .output()
            .expect("merge");

        assert!(
            cli.branch_is_merged("merged-branch", &default_branch)
                .await
                .expect("is_merged"),
            "merged branch should be detected as merged"
        );

        // Create an unmerged branch
        std::process::Command::new(&git)
            .args(["checkout", "-b", "unmerged-branch"])
            .current_dir(tmp.path())
            .output()
            .expect("checkout unmerged");
        std::fs::write(tmp.path().join("unmerged.txt"), "unmerged\n").unwrap();
        std::process::Command::new(&git)
            .args(["add", "unmerged.txt"])
            .current_dir(tmp.path())
            .output()
            .expect("git add");
        std::process::Command::new(&git)
            .args(["commit", "-m", "unmerged commit"])
            .current_dir(tmp.path())
            .output()
            .expect("git commit");

        // Go back to default branch (don't merge)
        std::process::Command::new(&git)
            .args(["checkout", &default_branch])
            .current_dir(tmp.path())
            .output()
            .expect("checkout default");

        assert!(
            !cli.branch_is_merged("unmerged-branch", &default_branch)
                .await
                .expect("not merged"),
            "unmerged branch should not be detected as merged"
        );
    }
}
