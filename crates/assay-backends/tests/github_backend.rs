#![cfg(feature = "github")]

//! Contract tests for `GitHubBackend` (red state).
//!
//! These tests define the complete `GitHubBackend` interface before the
//! implementation exists. They use a multi-subcommand mock `gh` shell script
//! with PATH override and `#[serial]` for isolation.
//!
//! Expected state: **will not compile** until `crate::github::GitHubBackend`
//! is implemented (T02). The compile error should be:
//!   "unresolved import `assay_backends::github`"

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serial_test::serial;
use tempfile::TempDir;
use tracing_test::traced_test;

use assay_core::{CapabilitySet, StateBackend};
use assay_types::{FailurePolicy, OrchestratorPhase, OrchestratorStatus};

// The module under test — will not compile until T02 implements it.
use assay_backends::github::GitHubBackend;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a minimal `OrchestratorStatus` for testing.
fn sample_status() -> OrchestratorStatus {
    OrchestratorStatus {
        run_id: "test-run-001".to_string(),
        phase: OrchestratorPhase::Running,
        failure_policy: FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: None,
        mesh_status: None,
        gossip_status: None,
    }
}

/// Write a mock `gh` script to `dir/gh` that dispatches on `$1 $2`.
///
/// `handlers` is a list of `(subcommand, behavior)` pairs where:
/// - `subcommand` is e.g. `"issue create"`, `"issue comment"`, `"issue view"`
/// - `behavior` is a shell fragment that runs when matched (e.g. `echo URL; exit 0`)
///
/// Unmatched subcommands cause the script to exit 127 with a diagnostic message.
fn write_mock_gh(dir: &Path, handlers: &[(&str, &str)]) {
    let mut script = String::from("#!/bin/sh\nCMD=\"$1 $2\"\n");
    for (subcmd, behavior) in handlers {
        script.push_str(&format!(
            "if [ \"$CMD\" = \"{subcmd}\" ]; then\n{behavior}\nfi\n"
        ));
    }
    script.push_str("echo \"mock gh: unhandled subcommand: $CMD\" >&2\nexit 127\n");

    let script_path = dir.join("gh");
    fs::write(&script_path, &script).expect("write mock gh script");
    let mut perms = fs::metadata(&script_path)
        .expect("read script metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set executable permission");
}

/// Prepend `dir` to `PATH`, run `f`, then restore original `PATH`.
///
/// # Safety
/// This modifies the process environment variable `PATH`. Tests using this
/// helper must be annotated with `#[serial]` to prevent concurrent access.
fn with_mock_gh_path<R, F: FnOnce() -> R>(dir: &Path, f: F) -> R {
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.display(), original_path);
    // SAFETY: guarded by #[serial] on all callers; no other threads modify PATH.
    unsafe { std::env::set_var("PATH", &new_path) };
    let result = f();
    unsafe { std::env::set_var("PATH", original_path) };
    result
}

/// Construct a `GitHubBackend` with test defaults.
fn make_backend() -> GitHubBackend {
    GitHubBackend::new("owner/repo".to_string(), Some("assay-run".to_string()))
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Capabilities should be all-false for GitHubBackend.
#[test]
fn test_capabilities_returns_none() {
    let backend = make_backend();
    let caps = backend.capabilities();
    assert_eq!(caps, CapabilitySet::none());
}

/// First `push_session_event` should invoke `gh issue create` and persist
/// the issue number parsed from the returned URL.
#[test]
#[serial]
fn test_push_first_event_creates_issue() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock: `gh issue create` prints a URL to stdout.
    // The script reads stdin (--body-file -) but ignores it for the mock.
    write_mock_gh(
        &bin_dir,
        &[(
            "issue create",
            "cat > /dev/null\necho 'https://github.com/owner/repo/issues/42'\nexit 0",
        )],
    );

    let backend = make_backend();
    let status = sample_status();

    with_mock_gh_path(&bin_dir, || {
        backend
            .push_session_event(&run_dir, &status)
            .expect("push_session_event should succeed");
    });

    // Verify issue number was persisted.
    let issue_file = run_dir.join(".github-issue-number");
    assert!(
        issue_file.exists(),
        ".github-issue-number should be created"
    );
    let number: u64 = fs::read_to_string(&issue_file)
        .unwrap()
        .trim()
        .parse()
        .expect("should contain a valid issue number");
    assert_eq!(number, 42);
}

/// Subsequent `push_session_event` calls (when `.github-issue-number` exists)
/// should invoke `gh issue comment` instead of creating a new issue.
#[test]
#[serial]
fn test_push_subsequent_event_creates_comment() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    // Pre-write the issue number file to simulate a prior push.
    fs::write(run_dir.join(".github-issue-number"), "42").unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock: `gh issue comment` reads stdin and exits 0.
    write_mock_gh(&bin_dir, &[("issue comment", "cat > /dev/null\nexit 0")]);

    let backend = make_backend();
    let status = sample_status();

    with_mock_gh_path(&bin_dir, || {
        backend
            .push_session_event(&run_dir, &status)
            .expect("push_session_event (comment) should succeed");
    });
}

/// `read_run_state` should deserialize the latest comment body as
/// `OrchestratorStatus` JSON when `.github-issue-number` exists.
#[test]
#[serial]
fn test_read_run_state_deserializes_latest_comment() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    // Pre-write the issue number file.
    fs::write(run_dir.join(".github-issue-number"), "42").unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Build the expected JSON that `gh issue view --json body,comments` returns.
    let status = sample_status();
    let status_json = serde_json::to_string(&status).unwrap();
    // Escape for shell embedding (single quotes).
    let escaped = status_json.replace('\'', "'\\''");

    let view_behavior = format!(
        "echo '{{\"body\":\"initial\",\"comments\":[{{\"body\":\"{escaped_for_json}\"}}]}}'\nexit 0",
        escaped_for_json = escaped.replace('\\', "\\\\").replace('"', "\\\""),
    );

    // Simpler approach: write the JSON to a temp file and cat it.
    let json_output = serde_json::json!({
        "body": "initial issue body",
        "comments": [
            { "body": status_json }
        ]
    });
    let json_file = tmp.path().join("view_output.json");
    fs::write(&json_file, json_output.to_string()).unwrap();

    let view_behavior = format!("cat '{}'\nexit 0", json_file.display());

    write_mock_gh(&bin_dir, &[("issue view", &view_behavior)]);

    let backend = make_backend();

    let result = with_mock_gh_path(&bin_dir, || backend.read_run_state(&run_dir));

    let state = result.expect("read_run_state should succeed");
    let state = state.expect("should return Some(status)");
    assert_eq!(state.run_id, "test-run-001");
    assert_eq!(state.phase, OrchestratorPhase::Running);
}

/// `read_run_state` returns `Ok(None)` when `.github-issue-number` doesn't exist.
#[test]
fn test_read_run_state_returns_none_without_issue_file() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let backend = make_backend();
    let result = backend
        .read_run_state(&run_dir)
        .expect("read_run_state should succeed");
    assert!(result.is_none(), "should return None without issue file");
}

/// `send_message` should return an error with `Unsupported` kind.
#[test]
fn test_send_message_returns_error() {
    let tmp = TempDir::new().unwrap();
    let backend = make_backend();
    let result = backend.send_message(tmp.path(), "test-msg", b"hello");
    assert!(result.is_err(), "send_message should return Err");
    let err = result.unwrap_err();
    let err_msg = format!("{err}");
    assert!(
        err_msg.to_lowercase().contains("unsupported")
            || err_msg.to_lowercase().contains("not supported"),
        "error should mention unsupported: {err_msg}"
    );
}

/// When `gh` is not on PATH, `push_session_event` should return an error.
#[test]
#[serial]
fn test_gh_not_found_returns_error() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    // Empty bin dir — no `gh` binary.
    let empty_dir = tmp.path().join("empty_bin");
    fs::create_dir_all(&empty_dir).unwrap();

    let backend = make_backend();
    let status = sample_status();

    // Set PATH to only the empty directory so `gh` is not found.
    let original_path = std::env::var("PATH").unwrap_or_default();
    unsafe { std::env::set_var("PATH", empty_dir.to_str().unwrap()) };

    let result = backend.push_session_event(&run_dir, &status);

    unsafe { std::env::set_var("PATH", original_path) };

    assert!(result.is_err(), "should error when gh is not found");
}

/// When `gh` exits with non-zero status, the error should include stderr.
#[test]
#[serial]
fn test_gh_nonzero_exit_returns_error() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock: `gh issue create` exits 1 with an error on stderr.
    write_mock_gh(
        &bin_dir,
        &[(
            "issue create",
            "cat > /dev/null\necho 'HTTP 401: Bad credentials' >&2\nexit 1",
        )],
    );

    let backend = make_backend();
    let status = sample_status();

    let result = with_mock_gh_path(&bin_dir, || backend.push_session_event(&run_dir, &status));

    assert!(result.is_err(), "should error on non-zero exit");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Bad credentials") || err_msg.contains("401"),
        "error should include stderr content: {err_msg}"
    );
}

// ── Q001: warn on malformed repo ─────────────────────────────────────────────

/// Q001: `GitHubBackend::new` should emit a `tracing::warn!` when repo is empty.
///
/// Currently fails (no warn emitted) — contract test for the fix in T02.
#[test]
#[traced_test]
fn test_new_warns_on_empty_repo() {
    let _backend = GitHubBackend::new("".to_string(), None);
    assert!(
        logs_contain("malformed"),
        "expected warn log containing 'malformed' for empty repo string"
    );
}

/// Q001: `GitHubBackend::new` should emit a `tracing::warn!` when repo has no slash.
///
/// A valid GitHub repo identifier must be `owner/repo`. A bare name like
/// `"noslash"` indicates a misconfiguration.
/// Currently fails (no warn emitted) — contract test for the fix in T02.
#[test]
#[traced_test]
fn test_new_warns_on_repo_missing_slash() {
    let _backend = GitHubBackend::new("noslash".to_string(), None);
    assert!(
        logs_contain("malformed"),
        "expected warn log containing 'malformed' for repo without '/'"
    );
}

// ── Q002: reject issue number 0 ──────────────────────────────────────────────

/// Q002: `read_issue_number` (called via `read_run_state`) should return `Err`
/// when `.github-issue-number` contains `"0"`.
///
/// Issue number 0 is invalid in the GitHub API. Accepting it silently causes
/// confusing downstream failures. The fix (T02) makes the parse return `Err`
/// with a message including "0".
/// Currently fails (0 is parsed as a valid `u64`) — contract test for the fix in T02.
#[test]
fn test_read_issue_number_rejects_zero() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    // Write "0" to the issue-number file.
    fs::write(run_dir.join(".github-issue-number"), "0").unwrap();

    let backend = make_backend();
    // `read_run_state` calls `read_issue_number` internally; the error propagates.
    let result = backend.read_run_state(&run_dir);

    assert!(
        result.is_err(),
        "read_run_state should return Err when issue number is 0, got: {result:?}"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains('0'),
        "error message should mention the invalid value '0': {err_msg}"
    );
}
