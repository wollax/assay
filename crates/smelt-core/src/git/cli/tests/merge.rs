use super::setup_test_repo;
use crate::error::SmeltError;
use crate::git::GitOps;
use crate::git::cli::GitCli;

#[tokio::test]
async fn test_merge_squash_clean() {
    let (tmp, cli) = setup_test_repo();
    let git = which::which("git").expect("git on PATH");
    let default_branch = cli.current_branch().await.expect("current_branch");

    // Create a feature branch with a commit
    std::process::Command::new(&git)
        .args(["checkout", "-b", "feature-squash"])
        .current_dir(tmp.path())
        .output()
        .expect("checkout feature");
    std::fs::write(tmp.path().join("feature.txt"), "feature content\n").unwrap();
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

    // Go back to default branch
    std::process::Command::new(&git)
        .args(["checkout", &default_branch])
        .current_dir(tmp.path())
        .output()
        .expect("checkout default");

    // Create a target branch and worktree for the merge
    cli.branch_create("merge-target", "HEAD")
        .await
        .expect("branch_create");
    let wt_path = tmp.path().parent().unwrap().join("smelt-test-squash-clean");
    cli.worktree_add_existing(&wt_path, "merge-target")
        .await
        .expect("worktree_add_existing");

    // Squash merge
    cli.merge_squash(&wt_path, "feature-squash")
        .await
        .expect("merge_squash should succeed");

    // Changes are staged but not committed — commit them
    let hash = cli
        .commit(&wt_path, "squash merge commit")
        .await
        .expect("commit after squash");
    assert!(
        hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()),
        "expected valid hash, got: {hash}"
    );

    // Verify the merge-target branch has the new commit
    let count = cli
        .rev_list_count("merge-target", &default_branch)
        .await
        .expect("rev_list_count");
    assert_eq!(
        count, 1,
        "merge-target should be 1 commit ahead after squash merge"
    );

    // Cleanup
    cli.worktree_remove(&wt_path, false)
        .await
        .expect("worktree_remove");
    let _ = std::fs::remove_dir_all(&wt_path);
}

#[tokio::test]
async fn test_merge_squash_conflict() {
    let (tmp, cli) = setup_test_repo();
    let git = which::which("git").expect("git on PATH");
    let default_branch = cli.current_branch().await.expect("current_branch");

    // Create branch-x that modifies README.md
    std::process::Command::new(&git)
        .args(["checkout", "-b", "branch-x"])
        .current_dir(tmp.path())
        .output()
        .expect("checkout branch-x");
    std::fs::write(tmp.path().join("README.md"), "branch-x content\n").unwrap();
    std::process::Command::new(&git)
        .args(["add", "README.md"])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    std::process::Command::new(&git)
        .args(["commit", "-m", "branch-x changes"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");

    // Go back to default and create branch-y that also modifies README.md differently
    std::process::Command::new(&git)
        .args(["checkout", &default_branch])
        .current_dir(tmp.path())
        .output()
        .expect("checkout default");
    std::process::Command::new(&git)
        .args(["checkout", "-b", "branch-y"])
        .current_dir(tmp.path())
        .output()
        .expect("checkout branch-y");
    std::fs::write(tmp.path().join("README.md"), "branch-y content\n").unwrap();
    std::process::Command::new(&git)
        .args(["add", "README.md"])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    std::process::Command::new(&git)
        .args(["commit", "-m", "branch-y changes"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");

    // Go back to default
    std::process::Command::new(&git)
        .args(["checkout", &default_branch])
        .current_dir(tmp.path())
        .output()
        .expect("checkout default");

    // Create target branch at branch-x HEAD (simulating first session already merged)
    // Then try to squash branch-y into it — conflict on README.md
    cli.branch_create("conflict-target", "branch-x")
        .await
        .expect("branch_create");
    let wt_path = tmp
        .path()
        .parent()
        .unwrap()
        .join(format!("smelt-test-squash-conflict-{}", std::process::id()));
    cli.worktree_add_existing(&wt_path, "conflict-target")
        .await
        .expect("worktree_add_existing");

    let result = cli.merge_squash(&wt_path, "branch-y").await;
    assert!(result.is_err(), "merge_squash should fail with conflict");

    let err = result.unwrap_err();
    match &err {
        SmeltError::MergeConflict { session, files } => {
            assert!(session.is_empty(), "GitOps should not set session name");
            assert!(
                files.contains(&"README.md".to_string()),
                "conflicting files should contain README.md, got: {files:?}"
            );
        }
        other => panic!("expected MergeConflict, got: {other:?}"),
    }

    // Cleanup: reset the worktree before removing
    cli.reset_hard(&wt_path, "HEAD").await.expect("reset_hard");
    cli.worktree_remove(&wt_path, true)
        .await
        .expect("worktree_remove");
    let _ = std::fs::remove_dir_all(&wt_path);
}

#[tokio::test]
async fn test_reset_hard() {
    let (tmp, cli) = setup_test_repo();
    let wt_path = tmp.path().parent().unwrap().join("smelt-test-reset-hard");

    cli.worktree_add(&wt_path, "reset-branch", "HEAD")
        .await
        .expect("worktree_add");

    // Make the worktree dirty
    std::fs::write(wt_path.join("dirty.txt"), "dirty\n").unwrap();
    let git = which::which("git").expect("git on PATH");
    std::process::Command::new(&git)
        .args(["add", "dirty.txt"])
        .current_dir(&wt_path)
        .output()
        .expect("git add");

    assert!(
        cli.worktree_is_dirty(&wt_path).await.expect("is_dirty"),
        "worktree should be dirty"
    );

    // Reset hard
    cli.reset_hard(&wt_path, "HEAD").await.expect("reset_hard");

    assert!(
        !cli.worktree_is_dirty(&wt_path)
            .await
            .expect("is_dirty after reset"),
        "worktree should be clean after reset --hard"
    );

    // Cleanup
    cli.worktree_remove(&wt_path, false)
        .await
        .expect("worktree_remove");
    let _ = std::fs::remove_dir_all(&wt_path);
}

#[tokio::test]
async fn test_unmerged_files_empty_when_clean() {
    let (tmp, cli) = setup_test_repo();

    let files = cli
        .unmerged_files(tmp.path())
        .await
        .expect("unmerged_files");
    assert!(
        files.is_empty(),
        "clean worktree should have no unmerged files"
    );
}

#[tokio::test]
async fn test_fetch_ref_creates_local_branch() {
    let git_bin = which::which("git").expect("git on PATH");

    // Step 1: create a bare remote repo
    let bare_dir = tempfile::tempdir().expect("bare temp dir");
    let status = std::process::Command::new(&git_bin)
        .args(["init", "--bare"])
        .current_dir(bare_dir.path())
        .output()
        .expect("git init --bare");
    assert!(status.status.success(), "git init --bare failed");

    // Step 2: clone the bare repo and push an initial commit
    let push_dir = tempfile::tempdir().expect("push temp dir");
    std::process::Command::new(&git_bin)
        .args(["clone", bare_dir.path().to_str().unwrap(), "."])
        .current_dir(push_dir.path())
        .output()
        .expect("git clone for push");
    for args in [
        &["config", "user.email", "test@example.com"][..],
        &["config", "user.name", "Test"][..],
    ] {
        std::process::Command::new(&git_bin)
            .args(args)
            .current_dir(push_dir.path())
            .output()
            .expect("git config");
    }
    std::fs::write(push_dir.path().join("file.txt"), "hello\n").unwrap();
    std::process::Command::new(&git_bin)
        .args(["add", "file.txt"])
        .current_dir(push_dir.path())
        .output()
        .expect("git add");
    std::process::Command::new(&git_bin)
        .args(["commit", "-m", "initial"])
        .current_dir(push_dir.path())
        .output()
        .expect("git commit");
    // Determine the default branch name
    let branch_output = std::process::Command::new(&git_bin)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(push_dir.path())
        .output()
        .expect("rev-parse HEAD");
    let default_branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();
    std::process::Command::new(&git_bin)
        .args(["push", "origin", &default_branch])
        .current_dir(push_dir.path())
        .output()
        .expect("git push");

    // Step 3: create a third clone (the working clone that will fetch)
    let work_dir = tempfile::tempdir().expect("work temp dir");
    std::process::Command::new(&git_bin)
        .args(["clone", bare_dir.path().to_str().unwrap(), "."])
        .current_dir(work_dir.path())
        .output()
        .expect("git clone for work");
    for args in [
        &["config", "user.email", "test@example.com"][..],
        &["config", "user.name", "Test"][..],
    ] {
        std::process::Command::new(&git_bin)
            .args(args)
            .current_dir(work_dir.path())
            .output()
            .expect("git config");
    }

    let git = GitCli::new(git_bin.clone(), work_dir.path().to_path_buf());

    // The local branch "fetched-main" does not exist yet
    assert!(
        !git.branch_exists("fetched-main")
            .await
            .expect("branch_exists before fetch"),
        "fetched-main should not exist before fetch_ref"
    );

    // fetch_ref with force-refspec creates the local branch directly
    let refspec = format!("+{}:{}", default_branch, "fetched-main");
    git.fetch_ref("origin", &refspec)
        .await
        .expect("fetch_ref should succeed");

    // After fetch_ref, the local branch exists
    assert!(
        git.branch_exists("fetched-main")
            .await
            .expect("branch_exists after fetch"),
        "fetched-main should exist after fetch_ref"
    );
}
