//! Integration tests for `smelt session run` CLI command.
//!
//! Each test creates a git repo as a subdirectory of the temp dir, so worktrees
//! land as siblings inside the temp dir and are cleaned up automatically.

use assert_cmd::Command;
use predicates::prelude::*;

/// Create a git repo at `tmp/test-repo/` with an initial commit and return the repo path.
fn setup_test_repo(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let repo_path = tmp.path().join("test-repo");
    std::fs::create_dir(&repo_path).expect("create repo dir");

    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&repo_path)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("HOME", tmp.path())
            .output()
            .expect("git command should run")
    };

    let out = git(&["init"]);
    assert!(out.status.success(), "git init failed");

    std::fs::write(repo_path.join("README.md"), "# test\n").unwrap();
    let out = git(&["add", "README.md"]);
    assert!(out.status.success(), "git add failed");
    let out = git(&["commit", "-m", "initial"]);
    assert!(out.status.success(), "git commit failed");

    repo_path
}

/// Write a manifest TOML file in the repo and return its path.
fn write_manifest(repo_path: &std::path::Path, content: &str) -> std::path::PathBuf {
    let manifest_path = repo_path.join("manifest.toml");
    std::fs::write(&manifest_path, content).expect("write manifest");
    manifest_path
}

/// Build a `Command` for the `smelt` binary with environment isolation.
fn smelt_cmd(dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("smelt").expect("binary should be built");
    cmd.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("HOME", dir);
    cmd
}

// ── Test 1: Basic 2-session manifest ─────────────────────────────────

#[test]
fn test_session_run_two_sessions_success() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let repo = setup_test_repo(&tmp);

    smelt_cmd(&repo).arg("init").assert().success();

    let manifest = write_manifest(
        &repo,
        r#"
[manifest]
name = "two-sessions"

[[session]]
name = "session-alpha"
task = "Write alpha file"

[session.script]
backend = "scripted"

[[session.script.steps]]
action = "commit"
message = "add alpha.txt"
files = [{ path = "alpha.txt", content = "alpha content\n" }]

[[session]]
name = "session-beta"
task = "Write beta file"

[session.script]
backend = "scripted"

[[session.script.steps]]
action = "commit"
message = "add beta.txt"
files = [{ path = "beta.txt", content = "beta content\n" }]
"#,
    );

    smelt_cmd(&repo)
        .args(["session", "run", manifest.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("2/2 sessions completed"))
        .stdout(predicate::str::contains("session-alpha: Completed"))
        .stdout(predicate::str::contains("session-beta: Completed"));

    // Verify worktree directories exist as siblings
    let wt_alpha = tmp.path().join("test-repo-smelt-session-alpha");
    let wt_beta = tmp.path().join("test-repo-smelt-session-beta");
    assert!(wt_alpha.exists(), "worktree for session-alpha should exist");
    assert!(wt_beta.exists(), "worktree for session-beta should exist");

    // Verify branches exist
    let output = std::process::Command::new("git")
        .args(["branch", "--list", "smelt/*"])
        .current_dir(&repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", tmp.path())
        .output()
        .expect("git branch --list");
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(
        branches.contains("smelt/session-alpha"),
        "branch smelt/session-alpha should exist"
    );
    assert!(
        branches.contains("smelt/session-beta"),
        "branch smelt/session-beta should exist"
    );
}

// ── Test 2: exit_after truncates execution ───────────────────────────

#[test]
fn test_session_run_exit_after_truncates() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let repo = setup_test_repo(&tmp);

    smelt_cmd(&repo).arg("init").assert().success();

    let manifest = write_manifest(
        &repo,
        r#"
[manifest]
name = "exit-after-test"

[[session]]
name = "truncated"
task = "Should stop after 1 step"

[session.script]
backend = "scripted"
exit_after = 1

[[session.script.steps]]
action = "commit"
message = "first"
files = [{ path = "a.txt", content = "a\n" }]

[[session.script.steps]]
action = "commit"
message = "second"
files = [{ path = "b.txt", content = "b\n" }]

[[session.script.steps]]
action = "commit"
message = "third"
files = [{ path = "c.txt", content = "c\n" }]
"#,
    );

    smelt_cmd(&repo)
        .args(["session", "run", manifest.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 steps"))
        .stdout(predicate::str::contains("1/1 sessions completed"));
}

// ── Test 3: simulate_failure = "crash" ──────────────────────────────

#[test]
fn test_session_run_simulate_failure_crash() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let repo = setup_test_repo(&tmp);

    smelt_cmd(&repo).arg("init").assert().success();

    let manifest = write_manifest(
        &repo,
        r#"
[manifest]
name = "crash-test"

[[session]]
name = "crasher"
task = "Will crash after 1 step"

[session.script]
backend = "scripted"
exit_after = 1
simulate_failure = "crash"

[[session.script.steps]]
action = "commit"
message = "first commit"
files = [{ path = "a.txt", content = "a\n" }]

[[session.script.steps]]
action = "commit"
message = "second commit"
files = [{ path = "b.txt", content = "b\n" }]
"#,
    );

    smelt_cmd(&repo)
        .args(["session", "run", manifest.to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Failed"))
        .stdout(predicate::str::contains("simulated crash"))
        .stdout(predicate::str::contains("0/1 sessions completed"));
}

// ── Test 4: Invalid manifest path ───────────────────────────────────

#[test]
fn test_session_run_invalid_manifest_path() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let repo = setup_test_repo(&tmp);

    smelt_cmd(&repo).arg("init").assert().success();

    smelt_cmd(&repo)
        .args(["session", "run", "nonexistent.toml"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Error"));
}

// ── Test 5: Conflict generation — two sessions editing same file ────

#[test]
fn test_session_run_conflict_same_file() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let repo = setup_test_repo(&tmp);

    smelt_cmd(&repo).arg("init").assert().success();

    let manifest = write_manifest(
        &repo,
        r#"
[manifest]
name = "conflict-test"

[[session]]
name = "writer-one"
task = "Write version one of shared.rs"

[session.script]
backend = "scripted"

[[session.script.steps]]
action = "commit"
message = "write shared.rs (version 1)"
files = [{ path = "src/shared.rs", content = "pub fn shared() -> &'static str { \"version-one\" }\n" }]

[[session]]
name = "writer-two"
task = "Write version two of shared.rs"

[session.script]
backend = "scripted"

[[session.script.steps]]
action = "commit"
message = "write shared.rs (version 2)"
files = [{ path = "src/shared.rs", content = "pub fn shared() -> &'static str { \"version-two\" }\n" }]
"#,
    );

    smelt_cmd(&repo)
        .args(["session", "run", manifest.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("2/2 sessions completed"));

    // Verify both worktrees exist with different content
    let wt_one = tmp.path().join("test-repo-smelt-writer-one");
    let wt_two = tmp.path().join("test-repo-smelt-writer-two");

    let content_one =
        std::fs::read_to_string(wt_one.join("src/shared.rs")).expect("read shared.rs from one");
    let content_two =
        std::fs::read_to_string(wt_two.join("src/shared.rs")).expect("read shared.rs from two");

    assert!(
        content_one.contains("version-one"),
        "writer-one should have version-one"
    );
    assert!(
        content_two.contains("version-two"),
        "writer-two should have version-two"
    );
    assert_ne!(content_one, content_two, "contents should differ");

    // Verify both branches exist
    let output = std::process::Command::new("git")
        .args(["branch", "--list", "smelt/*"])
        .current_dir(&repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", tmp.path())
        .output()
        .expect("git branch --list");
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(branches.contains("smelt/writer-one"));
    assert!(branches.contains("smelt/writer-two"));
}

// ── Test 6: Session run without init ────────────────────────────────

#[test]
fn test_session_run_without_init() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let repo = setup_test_repo(&tmp);

    // Do NOT run smelt init
    let manifest = write_manifest(
        &repo,
        r#"
[manifest]
name = "no-init-test"

[[session]]
name = "will-fail"
task = "Should fail without init"

[session.script]
backend = "scripted"

[[session.script.steps]]
action = "commit"
message = "nope"
files = [{ path = "a.txt", content = "a\n" }]
"#,
    );

    smelt_cmd(&repo)
        .args(["session", "run", manifest.to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not a Smelt project").or(
            predicate::str::contains("smelt init"),
        ));
}
