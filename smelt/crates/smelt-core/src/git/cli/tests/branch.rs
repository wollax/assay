use super::setup_test_repo;
use crate::git::GitOps;

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

#[tokio::test]
async fn test_merge_base() {
    let (tmp, cli) = setup_test_repo();
    let git = which::which("git").expect("git on PATH");
    let default_branch = cli.current_branch().await.expect("current_branch");

    // Record the common ancestor hash
    let base_hash = cli.rev_parse("HEAD").await.expect("rev_parse HEAD");

    // Create branch-a with a commit
    std::process::Command::new(&git)
        .args(["checkout", "-b", "branch-a"])
        .current_dir(tmp.path())
        .output()
        .expect("checkout branch-a");
    std::fs::write(tmp.path().join("a.txt"), "a\n").unwrap();
    std::process::Command::new(&git)
        .args(["add", "a.txt"])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    std::process::Command::new(&git)
        .args(["commit", "-m", "commit a"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");

    // Create branch-b from the same base
    std::process::Command::new(&git)
        .args(["checkout", &default_branch])
        .current_dir(tmp.path())
        .output()
        .expect("checkout default");
    std::process::Command::new(&git)
        .args(["checkout", "-b", "branch-b"])
        .current_dir(tmp.path())
        .output()
        .expect("checkout branch-b");
    std::fs::write(tmp.path().join("b.txt"), "b\n").unwrap();
    std::process::Command::new(&git)
        .args(["add", "b.txt"])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    std::process::Command::new(&git)
        .args(["commit", "-m", "commit b"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");

    // Go back to default
    std::process::Command::new(&git)
        .args(["checkout", &default_branch])
        .current_dir(tmp.path())
        .output()
        .expect("checkout default");

    let merge_base = cli
        .merge_base("branch-a", "branch-b")
        .await
        .expect("merge_base");
    assert_eq!(
        merge_base, base_hash,
        "merge-base should be the common ancestor"
    );
}

#[tokio::test]
async fn test_branch_create() {
    let (_tmp, cli) = setup_test_repo();

    cli.branch_create("new-branch", "HEAD")
        .await
        .expect("branch_create");
    assert!(
        cli.branch_exists("new-branch")
            .await
            .expect("branch_exists"),
        "newly created branch should exist"
    );
}
