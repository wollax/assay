use super::setup_test_repo;
use crate::git::GitOps;

#[tokio::test]
async fn test_add_empty_paths_returns_err() {
    let (tmp, cli) = setup_test_repo();
    let result = cli.add(tmp.path(), &[]).await;
    assert!(result.is_err(), "add() with no paths must return Err");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("at least one file path") || msg.contains("add"),
        "error should describe the constraint, got: {msg}"
    );
}

#[tokio::test]
async fn test_add_and_commit() {
    let (tmp, cli) = setup_test_repo();
    let default_branch = cli.current_branch().await.expect("current_branch");

    // Stage and commit a file via GitOps add()/commit()
    std::fs::write(tmp.path().join("new_file.txt"), "hello\n").unwrap();
    cli.add(tmp.path(), &["new_file.txt"]).await.expect("add");
    let hash = cli
        .commit(tmp.path(), "add new file")
        .await
        .expect("commit");

    // Verify hash is valid hex
    assert!(
        hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()),
        "expected short hex hash, got: {hash}"
    );

    // Verify rev_list_count reflects the commit just made
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
    let hash = cli.commit(tmp.path(), "test hash").await.expect("commit");

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
    let dirty = cli.worktree_is_dirty(tmp.path()).await.expect("is_dirty");
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
    cli.add(&wt_path, &["wt_file.txt"])
        .await
        .expect("add in wt");
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
async fn test_diff_numstat() {
    let (tmp, cli) = setup_test_repo();
    let git = which::which("git").expect("git on PATH");
    let default_branch = cli.current_branch().await.expect("current_branch");

    // Create a branch with changes
    std::process::Command::new(&git)
        .args(["checkout", "-b", "numstat-branch"])
        .current_dir(tmp.path())
        .output()
        .expect("checkout");
    std::fs::write(tmp.path().join("new_file.txt"), "line1\nline2\nline3\n").unwrap();
    std::process::Command::new(&git)
        .args(["add", "new_file.txt"])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    std::process::Command::new(&git)
        .args(["commit", "-m", "add new file"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");

    // Go back to default
    std::process::Command::new(&git)
        .args(["checkout", &default_branch])
        .current_dir(tmp.path())
        .output()
        .expect("checkout default");

    let stats = cli
        .diff_numstat(&default_branch, "numstat-branch")
        .await
        .expect("diff_numstat");

    assert!(!stats.is_empty(), "should have diff stats");
    let new_file_stat = stats.iter().find(|(_, _, name)| name == "new_file.txt");
    assert!(new_file_stat.is_some(), "should find new_file.txt in stats");
    let (ins, del, _) = new_file_stat.unwrap();
    assert_eq!(*ins, 3, "should have 3 insertions");
    assert_eq!(*del, 0, "should have 0 deletions");
}

#[tokio::test]
async fn test_diff_name_only() {
    let (tmp, cli) = setup_test_repo();
    let git = which::which("git").expect("git on PATH");
    let default_branch = cli.current_branch().await.expect("current_branch");

    // Create a feature branch with changes
    std::process::Command::new(&git)
        .args(["checkout", "-b", "name-only-branch"])
        .current_dir(tmp.path())
        .output()
        .expect("checkout");
    std::fs::write(tmp.path().join("file_a.txt"), "a\n").unwrap();
    std::fs::write(tmp.path().join("file_b.txt"), "b\n").unwrap();
    std::process::Command::new(&git)
        .args(["add", "file_a.txt", "file_b.txt"])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    std::process::Command::new(&git)
        .args(["commit", "-m", "add files"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");

    // Go back to default
    std::process::Command::new(&git)
        .args(["checkout", &default_branch])
        .current_dir(tmp.path())
        .output()
        .expect("checkout default");

    let files = cli
        .diff_name_only(&default_branch, "name-only-branch")
        .await
        .expect("diff_name_only");

    assert_eq!(files.len(), 2, "should have 2 changed files");
    assert!(files.contains(&"file_a.txt".to_string()));
    assert!(files.contains(&"file_b.txt".to_string()));
}

#[tokio::test]
async fn test_diff_name_only_empty() {
    let (_tmp, cli) = setup_test_repo();
    let default_branch = cli.current_branch().await.expect("current_branch");

    let files = cli
        .diff_name_only(&default_branch, &default_branch)
        .await
        .expect("diff_name_only same ref");

    assert!(files.is_empty(), "same ref should have no diff");
}

#[tokio::test]
async fn test_log_subjects() {
    let (tmp, cli) = setup_test_repo();
    let git = which::which("git").expect("git on PATH");

    // Create 2 additional commits with known subjects
    for (i, subject) in ["Add feature A", "Fix bug B"].iter().enumerate() {
        std::fs::write(
            tmp.path().join(format!("log_test_{i}.txt")),
            format!("content {i}\n"),
        )
        .unwrap();
        std::process::Command::new(&git)
            .args(["add", &format!("log_test_{i}.txt")])
            .current_dir(tmp.path())
            .output()
            .expect("git add");
        std::process::Command::new(&git)
            .args(["commit", "-m", subject])
            .current_dir(tmp.path())
            .output()
            .expect("git commit");
    }

    let subjects = cli
        .log_subjects("HEAD~2..HEAD")
        .await
        .expect("log_subjects");

    assert_eq!(subjects.len(), 2, "should have 2 commit subjects");
    // git log returns newest first
    assert_eq!(subjects[0], "Fix bug B");
    assert_eq!(subjects[1], "Add feature A");
}

#[tokio::test]
async fn test_log_subjects_empty_range() {
    let (_tmp, cli) = setup_test_repo();

    let subjects = cli
        .log_subjects("HEAD..HEAD")
        .await
        .expect("log_subjects empty");

    assert!(subjects.is_empty(), "same ref range should be empty");
}
