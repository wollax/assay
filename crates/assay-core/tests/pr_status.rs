//! Integration tests for `assay_core::pr::pr_status_poll`.
//!
//! Uses the same `write_fake_gh` / `with_mock_gh_path` pattern from the
//! existing `pr.rs` tests to verify parsing for all status combinations.

use serial_test::serial;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

use assay_core::pr::{PrStatusState, pr_status_poll};

// ── Helpers (duplicated from tests/pr.rs — those are private) ────────────────

/// Write a fake `gh` script to `dir/gh` that prints `stdout` and exits with `exit_code`.
fn write_fake_gh(dir: &Path, exit_code: i32, stdout: &str) {
    let script_path = dir.join("gh");
    let script = format!(
        "#!/bin/sh\necho '{}'\nexit {}\n",
        stdout.replace('\'', "'\\''"),
        exit_code
    );
    fs::write(&script_path, script).expect("write fake gh script");
    let mut perms = fs::metadata(&script_path)
        .expect("read script metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set executable permission");
}

/// Write a fake `gh` script that writes to stderr and exits with `exit_code`.
fn write_fake_gh_stderr(dir: &Path, exit_code: i32, stderr_msg: &str) {
    let script_path = dir.join("gh");
    let script = format!(
        "#!/bin/sh\necho '{}' >&2\nexit {}\n",
        stderr_msg.replace('\'', "'\\''"),
        exit_code
    );
    fs::write(&script_path, script).expect("write fake gh script");
    let mut perms = fs::metadata(&script_path)
        .expect("read script metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set executable permission");
}

/// Prepend `dir` to `PATH`, run `f`, then restore original `PATH`.
///
/// # Safety
/// Modifies the process environment. Callers must use `#[serial]`.
fn with_mock_gh_path<R, F: FnOnce() -> R>(dir: &Path, f: F) -> R {
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.display(), original_path);
    unsafe { std::env::set_var("PATH", &new_path) };
    let result = f();
    unsafe { std::env::set_var("PATH", original_path) };
    result
}

// ── Test 1: OPEN with passing checks and APPROVED ────────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_open_with_passing_checks() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let json = r#"{"state":"OPEN","statusCheckRollup":[{"conclusion":"SUCCESS","status":"COMPLETED","name":"CI"}],"reviewDecision":"APPROVED"}"#;
    write_fake_gh(&bin_dir, 0, json);

    let result = with_mock_gh_path(&bin_dir, || pr_status_poll(123));
    let info = result.expect("should parse successfully");

    assert_eq!(info.state, PrStatusState::Open);
    assert_eq!(info.ci_pass, 1);
    assert_eq!(info.ci_fail, 0);
    assert_eq!(info.ci_pending, 0);
    assert_eq!(info.review_decision, "APPROVED");
}

// ── Test 2: MERGED with no checks ───────────────────────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_merged_no_checks() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let json = r#"{"state":"MERGED","statusCheckRollup":[],"reviewDecision":""}"#;
    write_fake_gh(&bin_dir, 0, json);

    let result = with_mock_gh_path(&bin_dir, || pr_status_poll(456));
    let info = result.expect("should parse successfully");

    assert_eq!(info.state, PrStatusState::Merged);
    assert_eq!(info.ci_pass, 0);
    assert_eq!(info.ci_fail, 0);
    assert_eq!(info.ci_pending, 0);
    assert_eq!(info.review_decision, "");
}

// ── Test 3: OPEN with mixed checks ──────────────────────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_open_with_mixed_checks() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let json = r#"{"state":"OPEN","statusCheckRollup":[{"conclusion":"SUCCESS","status":"COMPLETED","name":"build"},{"conclusion":"SUCCESS","status":"COMPLETED","name":"lint"},{"conclusion":"FAILURE","status":"COMPLETED","name":"test"},{"conclusion":null,"status":"IN_PROGRESS","name":"deploy"}],"reviewDecision":"CHANGES_REQUESTED"}"#;
    write_fake_gh(&bin_dir, 0, json);

    let result = with_mock_gh_path(&bin_dir, || pr_status_poll(789));
    let info = result.expect("should parse successfully");

    assert_eq!(info.state, PrStatusState::Open);
    assert_eq!(info.ci_pass, 2);
    assert_eq!(info.ci_fail, 1);
    assert_eq!(info.ci_pending, 1);
    assert_eq!(info.review_decision, "CHANGES_REQUESTED");
}

// ── Test 4: gh not found → Err ──────────────────────────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_gh_not_found() {
    let tmp = TempDir::new().expect("tempdir");
    let empty_dir = tmp.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();

    // Replace PATH entirely so `gh` is not found
    let original_path = std::env::var("PATH").unwrap_or_default();
    unsafe { std::env::set_var("PATH", empty_dir.display().to_string()) };

    let result = pr_status_poll(999);

    unsafe { std::env::set_var("PATH", original_path) };

    assert!(result.is_err(), "expected Err when gh not in PATH");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("gh CLI not found") || msg.contains("No such file"),
        "error should mention gh not found, got: {msg}"
    );
}

// ── Test 5: CLOSED state ────────────────────────────────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_closed() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let json = r#"{"state":"CLOSED","statusCheckRollup":null,"reviewDecision":"REVIEW_REQUIRED"}"#;
    write_fake_gh(&bin_dir, 0, json);

    let result = with_mock_gh_path(&bin_dir, || pr_status_poll(555));
    let info = result.expect("should parse successfully");

    assert_eq!(info.state, PrStatusState::Closed);
    assert_eq!(info.ci_pass, 0);
    assert_eq!(info.ci_fail, 0);
    assert_eq!(info.ci_pending, 0);
    assert_eq!(info.review_decision, "REVIEW_REQUIRED");
}

// ── Test 6: Non-zero exit code → Err with stderr ────────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_gh_nonzero_exit() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    write_fake_gh_stderr(&bin_dir, 1, "GraphQL: Could not resolve to a PullRequest");

    let result = with_mock_gh_path(&bin_dir, || pr_status_poll(404));

    assert!(result.is_err(), "expected Err on non-zero exit");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Could not resolve") || msg.contains("exit"),
        "error should include stderr, got: {msg}"
    );
}

// ── Test 7: Malformed JSON → Err ────────────────────────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_malformed_json() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    write_fake_gh(&bin_dir, 0, "not valid json at all");

    let result = with_mock_gh_path(&bin_dir, || pr_status_poll(111));

    assert!(result.is_err(), "expected Err on malformed JSON");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("parse") || msg.contains("JSON"),
        "error should mention parse failure, got: {msg}"
    );
}

// ── Test 8: CANCELLED conclusion counts as failure ──────────────────────────

#[test]
#[serial]
fn test_pr_status_poll_cancelled_counts_as_fail() {
    let tmp = TempDir::new().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let json = r#"{"state":"OPEN","statusCheckRollup":[{"conclusion":"CANCELLED","status":"COMPLETED","name":"ci"}],"reviewDecision":""}"#;
    write_fake_gh(&bin_dir, 0, json);

    let result = with_mock_gh_path(&bin_dir, || pr_status_poll(222));
    let info = result.expect("should parse successfully");

    assert_eq!(info.ci_fail, 1, "CANCELLED should count as failure");
    assert_eq!(info.ci_pass, 0);
    assert_eq!(info.ci_pending, 0);
}
