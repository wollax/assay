use super::setup_test_repo;
use crate::git::GitOps;

/// Returns a unique worktree path under `parent` with the given label suffix.
/// Uniqueness is ensured by combining the OS process ID (stable for the whole
/// test run) with a sub-second nanosecond timestamp (distinguishes tests within
/// the same process). The two together make collisions across parallel test
/// threads effectively impossible.
fn unique_wt_path(parent: &std::path::Path, label: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    parent.join(format!("smelt-test-wt-{label}-{pid}-{ts}"))
}

#[tokio::test]
async fn test_worktree_add_and_list() {
    let (tmp, cli) = setup_test_repo();
    let wt_path = unique_wt_path(tmp.path().parent().unwrap(), "add");

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
    let wt_path = unique_wt_path(tmp.path().parent().unwrap(), "remove");

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
async fn test_worktree_is_dirty() {
    let (tmp, cli) = setup_test_repo();
    let wt_path = unique_wt_path(tmp.path().parent().unwrap(), "dirty");

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
async fn test_worktree_add_existing() {
    let (tmp, cli) = setup_test_repo();
    let git = which::which("git").expect("git on PATH");

    // Create a branch (not checked out)
    std::process::Command::new(&git)
        .args(["branch", "existing-branch"])
        .current_dir(tmp.path())
        .output()
        .expect("create branch");

    let wt_path = unique_wt_path(tmp.path().parent().unwrap(), "existing");
    cli.worktree_add_existing(&wt_path, "existing-branch")
        .await
        .expect("worktree_add_existing");

    // Verify worktree exists
    assert!(wt_path.exists(), "worktree directory should exist");

    // Verify it's on the correct branch
    let output = std::process::Command::new(&git)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&wt_path)
        .output()
        .expect("rev-parse in worktree");
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(
        branch, "existing-branch",
        "worktree should be on existing-branch"
    );

    // Cleanup
    cli.worktree_remove(&wt_path, false)
        .await
        .expect("worktree_remove");
    let _ = std::fs::remove_dir_all(&wt_path);
}
