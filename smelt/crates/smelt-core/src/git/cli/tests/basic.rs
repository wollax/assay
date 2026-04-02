use super::setup_test_repo;
use crate::git::GitOps;

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
    // Short hash is at least 7 hex characters (length varies with repo size and git config)
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
async fn test_rev_parse() {
    let (_tmp, cli) = setup_test_repo();
    let hash = cli.rev_parse("HEAD").await.expect("rev_parse");

    assert_eq!(hash.len(), 40, "full hash should be 40 chars, got: {hash}");
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "hash should be hex, got: {hash}",
    );
}
