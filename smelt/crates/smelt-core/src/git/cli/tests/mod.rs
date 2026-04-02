use super::*;

mod basic;
mod branch;
mod commit;
mod merge;
mod worktree;

/// Create a temporary git repo with an initial commit, returning (temp_dir, GitCli).
pub(super) fn setup_test_repo() -> (tempfile::TempDir, GitCli) {
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
        let out = std::process::Command::new(&git)
            .args(args)
            .current_dir(tmp.path())
            .output()
            .expect("git config: failed to spawn");
        assert!(
            out.status.success(),
            "git config {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // Create initial commit
    std::fs::write(tmp.path().join("README.md"), "# test\n").unwrap();
    let out = std::process::Command::new(&git)
        .args(["add", "README.md"])
        .current_dir(tmp.path())
        .output()
        .expect("git add: failed to spawn");
    assert!(
        out.status.success(),
        "git add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = std::process::Command::new(&git)
        .args(["commit", "-m", "initial"])
        .current_dir(tmp.path())
        .output()
        .expect("git commit: failed to spawn");
    assert!(
        out.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let cli = GitCli::new(git, tmp.path().to_path_buf());
    (tmp, cli)
}
