//! Result collection: reads git state after Assay completes and creates target branch.

use std::path::PathBuf;

use crate::error::Result;
use crate::git::GitOps;

/// Structured result of collecting commits onto a target branch.
#[derive(Debug, Clone)]
pub struct BranchCollectResult {
    /// Name of the target branch created/updated.
    pub branch: String,
    /// Number of commits between base_ref and HEAD.
    pub commit_count: usize,
    /// File paths changed between base_ref and HEAD.
    pub files_changed: Vec<String>,
    /// Commit subject lines between base_ref and HEAD.
    pub subjects: Vec<String>,
    /// True when HEAD == base_ref (no new commits).
    pub no_changes: bool,
}

/// Collects Assay results from the git repo and creates a target branch.
///
/// Generic over `G: GitOps` for testability — no Docker dependency.
pub struct ResultCollector<G: GitOps> {
    git: G,
    repo_path: PathBuf,
}

impl<G: GitOps> ResultCollector<G> {
    /// Create a new collector for the given repo.
    pub fn new(git: G, repo_path: PathBuf) -> Self {
        Self { git, repo_path }
    }

    /// Collect commits since `base_ref` and create/update `target_branch` pointing at HEAD.
    ///
    /// Returns a [`BranchCollectResult`] describing what was collected.
    pub async fn collect(
        &self,
        base_ref: &str,
        target_branch: &str,
    ) -> Result<BranchCollectResult> {
        let head = self.git.rev_parse("HEAD").await?;
        tracing::info!(hash = %head, "HEAD at");

        let base = self.git.rev_parse(base_ref).await?;
        tracing::info!(hash = %base, "base_ref at");

        // No new commits — early return without creating a branch.
        if head == base {
            tracing::warn!("no new commits between HEAD and base_ref");
            return Ok(BranchCollectResult {
                branch: target_branch.to_string(),
                commit_count: 0,
                files_changed: Vec::new(),
                subjects: Vec::new(),
                no_changes: true,
            });
        }

        // Check for uncommitted changes.
        if self.git.worktree_is_dirty(&self.repo_path).await? {
            tracing::warn!("dirty working tree detected — uncommitted changes present");
        }

        let commit_count = self.git.rev_list_count("HEAD", base_ref).await?;
        tracing::info!(count = commit_count, "commits to collect");

        let files_changed = self.git.diff_name_only(base_ref, "HEAD").await?;
        let subjects = self.git.log_subjects(&format!("{base_ref}..HEAD")).await?;

        // Handle pre-existing target branch.
        if self.git.branch_exists(target_branch).await? {
            let old_hash = self.git.rev_parse(target_branch).await?;
            tracing::warn!(
                old_hash = %old_hash,
                new_hash = %head,
                "target branch '{}' already exists, updating",
                target_branch
            );
            self.git.branch_delete(target_branch, true).await?;
        }

        self.git.branch_create(target_branch, "HEAD").await?;
        tracing::info!(
            branch = target_branch,
            hash = %head,
            "target branch created"
        );

        Ok(BranchCollectResult {
            branch: target_branch.to_string(),
            commit_count,
            files_changed,
            subjects,
            no_changes: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitCli;

    /// Create a temporary git repo with an initial commit, returning (temp_dir, GitCli).
    fn setup_test_repo() -> (tempfile::TempDir, GitCli) {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let git = which::which("git").expect("git on PATH");

        let run = |args: &[&str]| {
            let out = std::process::Command::new(&git)
                .args(args)
                .current_dir(tmp.path())
                .output()
                .expect("git command");
            assert!(
                out.status.success(),
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        };

        run(&["init"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);

        std::fs::write(tmp.path().join("README.md"), "# test\n").unwrap();
        run(&["add", "README.md"]);
        run(&["commit", "-m", "initial"]);

        let cli = GitCli::new(git, tmp.path().to_path_buf());
        (tmp, cli)
    }

    /// Helper: create a file, stage, and commit.
    fn add_commit(tmp: &std::path::Path, filename: &str, content: &str, message: &str) {
        let git = which::which("git").expect("git on PATH");
        std::fs::write(tmp.join(filename), content).unwrap();
        let run = |args: &[&str]| {
            let out = std::process::Command::new(&git)
                .args(args)
                .current_dir(tmp)
                .output()
                .expect("git command");
            assert!(
                out.status.success(),
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["add", filename]);
        run(&["commit", "-m", message]);
    }

    /// Helper: get current HEAD hash.
    fn head_hash(tmp: &std::path::Path) -> String {
        let git = which::which("git").expect("git on PATH");
        let out = std::process::Command::new(&git)
            .args(["rev-parse", "HEAD"])
            .current_dir(tmp)
            .output()
            .expect("rev-parse HEAD");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[tokio::test]
    async fn test_collect_basic() {
        let (tmp, cli) = setup_test_repo();
        let base = head_hash(tmp.path());

        add_commit(tmp.path(), "foo.txt", "hello", "add foo");

        let collector = ResultCollector::new(cli, tmp.path().to_path_buf());
        let result = collector.collect(&base, "results/test").await.unwrap();

        assert!(!result.no_changes);
        assert_eq!(result.commit_count, 1);
        assert_eq!(result.branch, "results/test");
        assert!(result.files_changed.contains(&"foo.txt".to_string()));
        assert_eq!(result.subjects.len(), 1);
        assert!(result.subjects[0].contains("add foo"));

        // Verify branch was created pointing at HEAD.
        let current_head = head_hash(tmp.path());
        let branch_hash = {
            let git = which::which("git").unwrap();
            let out = std::process::Command::new(&git)
                .args(["rev-parse", "results/test"])
                .current_dir(tmp.path())
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        assert_eq!(branch_hash, current_head);
    }

    #[tokio::test]
    async fn test_collect_no_changes() {
        let (tmp, cli) = setup_test_repo();
        let base = head_hash(tmp.path());

        let collector = ResultCollector::new(cli, tmp.path().to_path_buf());
        let result = collector.collect(&base, "results/empty").await.unwrap();

        assert!(result.no_changes);
        assert_eq!(result.commit_count, 0);
        assert!(result.files_changed.is_empty());
        assert!(result.subjects.is_empty());

        // Branch should NOT have been created.
        let git = which::which("git").unwrap();
        let out = std::process::Command::new(&git)
            .args(["rev-parse", "--verify", "results/empty"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(!out.status.success(), "branch should not exist");
    }

    #[tokio::test]
    async fn test_collect_target_already_exists() {
        let (tmp, cli) = setup_test_repo();
        let base = head_hash(tmp.path());

        // Create target branch at the initial commit (old position).
        let git_bin = which::which("git").unwrap();
        let run = |args: &[&str]| {
            std::process::Command::new(&git_bin)
                .args(args)
                .current_dir(tmp.path())
                .output()
                .expect("git command");
        };
        run(&["branch", "results/existing", "HEAD"]);

        // Add a new commit so HEAD moves ahead.
        add_commit(tmp.path(), "bar.txt", "data", "add bar");
        let new_head = head_hash(tmp.path());

        let collector = ResultCollector::new(cli, tmp.path().to_path_buf());
        let result = collector
            .collect(&base, "results/existing")
            .await
            .unwrap();

        assert!(!result.no_changes);
        assert_eq!(result.commit_count, 1);

        // Branch should now point at the new HEAD.
        let out = std::process::Command::new(&git_bin)
            .args(["rev-parse", "results/existing"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        let branch_hash = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert_eq!(branch_hash, new_head);
    }

    #[tokio::test]
    async fn test_collect_multiple_commits() {
        let (tmp, cli) = setup_test_repo();
        let base = head_hash(tmp.path());

        add_commit(tmp.path(), "a.txt", "a", "first commit");
        add_commit(tmp.path(), "b.txt", "b", "second commit");
        add_commit(tmp.path(), "c.txt", "c", "third commit");

        let collector = ResultCollector::new(cli, tmp.path().to_path_buf());
        let result = collector.collect(&base, "results/multi").await.unwrap();

        assert!(!result.no_changes);
        assert_eq!(result.commit_count, 3);
        assert_eq!(result.subjects.len(), 3);
        assert!(result.files_changed.contains(&"a.txt".to_string()));
        assert!(result.files_changed.contains(&"b.txt".to_string()));
        assert!(result.files_changed.contains(&"c.txt".to_string()));
    }

    #[tokio::test]
    async fn test_collect_dirty_worktree() {
        let (tmp, cli) = setup_test_repo();
        let base = head_hash(tmp.path());

        add_commit(tmp.path(), "clean.txt", "clean", "committed file");

        // Create uncommitted (dirty) changes.
        std::fs::write(tmp.path().join("dirty.txt"), "uncommitted").unwrap();

        let collector = ResultCollector::new(cli, tmp.path().to_path_buf());
        let result = collector.collect(&base, "results/dirty").await.unwrap();

        // Should still collect the committed changes successfully.
        assert!(!result.no_changes);
        assert_eq!(result.commit_count, 1);
        assert!(result.files_changed.contains(&"clean.txt".to_string()));
        // dirty.txt is not committed, so shouldn't appear in files_changed.
        assert!(!result.files_changed.contains(&"dirty.txt".to_string()));
    }
}
