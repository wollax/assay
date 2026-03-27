#![cfg(feature = "ssh")]

//! Contract tests for `SshSyncBackend` (red state).
//!
//! These tests define the complete `SshSyncBackend` interface before the
//! implementation exists. They use mock `scp`/`ssh` shell scripts with PATH
//! override and `#[serial]` for isolation.
//!
//! Expected state: **will not compile** until `crate::ssh::SshSyncBackend`
//! is implemented (T02). The compile error should be:
//!   "unresolved import `assay_backends::ssh`"

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serial_test::serial;
use tempfile::TempDir;

use assay_core::{CapabilitySet, StateBackend};
use assay_types::{FailurePolicy, OrchestratorPhase, OrchestratorStatus};

// The module under test — will not compile until T02 implements it.
use assay_backends::ssh::SshSyncBackend;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a minimal `OrchestratorStatus` for testing.
fn sample_status() -> OrchestratorStatus {
    OrchestratorStatus {
        run_id: "test-run-ssh-001".to_string(),
        phase: OrchestratorPhase::Running,
        failure_policy: FailurePolicy::SkipDependents,
        sessions: vec![],
        started_at: chrono::Utc::now(),
        completed_at: None,
        mesh_status: None,
        gossip_status: None,
    }
}

/// Write a mock `scp` script to `dir/scp`.
///
/// The script distinguishes push from pull by inspecting the last two
/// positional arguments:
/// - **Push:** second-to-last is a local path, last contains `:` (remote spec)
/// - **Pull:** first non-flag arg contains `:` (remote spec), last is local
///
/// `on_push` and `on_pull` are shell fragments executed for each case.
fn write_mock_scp(dir: &Path, on_push: &str, on_pull: &str) {
    let script = format!(
        r#"#!/bin/sh
# Collect non-flag arguments
ARGS=""
for arg in "$@"; do
    case "$arg" in
        -*) ;;
        *) ARGS="$ARGS $arg" ;;
    esac
done

# Get the last two positional (non-flag) arguments
LAST=""
SECOND_LAST=""
for arg in $ARGS; do
    SECOND_LAST="$LAST"
    LAST="$arg"
done

# If last arg contains ':' -> push (local to remote)
# If second-to-last arg contains ':' -> pull (remote to local)
case "$LAST" in
    *:*)
        # Push: local -> remote
{on_push}
        ;;
    *)
        case "$SECOND_LAST" in
            *:*)
                # Pull: remote -> local
{on_pull}
                ;;
            *)
                echo "mock scp: cannot determine direction" >&2
                exit 1
                ;;
        esac
        ;;
esac
"#
    );

    let script_path = dir.join("scp");
    fs::write(&script_path, &script).expect("write mock scp script");
    let mut perms = fs::metadata(&script_path)
        .expect("read script metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("set executable permission");
}

/// Write a mock `ssh` script to `dir/ssh`.
///
/// Takes the last argument as the remote command, dispatches on prefix
/// matches from `cmd_handlers`. Unhandled commands exit 127.
fn write_mock_ssh(dir: &Path, cmd_handlers: &[(&str, &str)]) {
    let mut script =
        String::from("#!/bin/sh\n# Last argument is the remote command\nCMD=\"${@: -1}\"\n");
    for (prefix, behavior) in cmd_handlers {
        script.push_str(&format!(
            "case \"$CMD\" in\n{prefix}*)\n{behavior}\n;;\nesac\n"
        ));
    }
    script.push_str("echo \"mock ssh: unhandled command: $CMD\" >&2\nexit 127\n");

    let script_path = dir.join("ssh");
    fs::write(&script_path, &script).expect("write mock ssh script");
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
fn with_mock_path<R, F: FnOnce() -> R>(dir: &Path, f: F) -> R {
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.display(), original_path);
    // SAFETY: guarded by #[serial] on all callers; no other threads modify PATH.
    unsafe { std::env::set_var("PATH", &new_path) };
    let result = f();
    unsafe { std::env::set_var("PATH", original_path) };
    result
}

// ── Tests: Capabilities and Push/Pull ────────────────────────────────────────

/// Capabilities should be all-true for SshSyncBackend (full filesystem semantics).
#[test]
#[serial]
fn test_capabilities_returns_all() {
    let tmp = TempDir::new().unwrap();
    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );
    assert_eq!(backend.capabilities(), CapabilitySet::all());
}

/// `push_session_event` should invoke `scp` to push state.json to remote.
#[test]
#[serial]
fn test_push_session_event_scp_args() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let marker_dir = tmp.path().join("markers");
    fs::create_dir_all(&marker_dir).unwrap();
    let marker_file = marker_dir.join("push_called");

    // Mock scp: on push, create marker file; on pull, exit 1.
    let on_push = format!("        touch '{}'\n        exit 0", marker_file.display());
    write_mock_scp(&bin_dir, &on_push, "        exit 1");

    // Mock ssh: mkdir exits 0
    write_mock_ssh(&bin_dir, &[("mkdir", "    exit 0")]);

    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );
    let status = sample_status();

    with_mock_path(&bin_dir, || {
        backend
            .push_session_event(&run_dir, &status)
            .expect("push_session_event should succeed");
    });

    assert!(
        marker_file.exists(),
        "push_called marker should have been created by mock scp"
    );
}

/// `read_run_state` should deserialize the pulled state.json into `OrchestratorStatus`.
#[test]
#[serial]
fn test_read_run_state_returns_deserialized_status() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Build the status JSON to be returned by the mock scp pull.
    let status = sample_status();
    let status_json = serde_json::to_string(&status).unwrap();
    let status_json_file = tmp.path().join("status_payload.json");
    fs::write(&status_json_file, &status_json).unwrap();

    // Mock scp: on pull, copy the payload to the destination (last arg).
    let on_pull = format!(
        "        cp '{}' \"$LAST\"\n        exit 0",
        status_json_file.display()
    );
    write_mock_scp(&bin_dir, "        exit 1", &on_pull);

    // Mock ssh not needed for read, but provide it for completeness.
    write_mock_ssh(&bin_dir, &[]);

    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );

    let result = with_mock_path(&bin_dir, || backend.read_run_state(&run_dir));

    let state = result.expect("read_run_state should succeed");
    let state = state.expect("should return Some(status)");
    assert_eq!(state.run_id, "test-run-ssh-001");
    assert_eq!(state.phase, OrchestratorPhase::Running);
}

/// `read_run_state` returns `Ok(None)` when the remote file doesn't exist
/// (simulated by scp returning non-zero).
#[test]
#[serial]
fn test_read_run_state_returns_none_when_file_missing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock scp: on pull, exit 1 (file not found).
    write_mock_scp(&bin_dir, "        exit 1", "        exit 1");
    write_mock_ssh(&bin_dir, &[]);

    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );

    let result = with_mock_path(&bin_dir, || backend.read_run_state(&run_dir));

    let state = result.expect("read_run_state should succeed");
    assert!(
        state.is_none(),
        "should return None when remote file is missing"
    );
}

// ── Tests: Messaging, Annotations, Checkpoints ──────────────────────────────

/// `send_message` pushes a message file to the remote inbox.
#[test]
#[serial]
fn test_send_message_pushes_to_remote_inbox() {
    let tmp = TempDir::new().unwrap();
    let inbox_path = tmp.path().join("run").join("inbox");
    fs::create_dir_all(&inbox_path).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let marker_file = tmp.path().join("push_called");
    let on_push = format!("        touch '{}'\n        exit 0", marker_file.display());
    write_mock_scp(&bin_dir, &on_push, "        exit 1");
    write_mock_ssh(&bin_dir, &[("mkdir", "    exit 0")]);

    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );

    let result = with_mock_path(&bin_dir, || {
        backend.send_message(&inbox_path, "msg.json", b"hello")
    });

    result.expect("send_message should succeed");
    assert!(
        marker_file.exists(),
        "push_called marker should have been created"
    );
}

/// `poll_inbox` pulls remote inbox listing, fetches each file, and removes remote copies.
#[test]
#[serial]
fn test_poll_inbox_pulls_and_removes_remote_files() {
    let tmp = TempDir::new().unwrap();
    let inbox_path = tmp.path().join("run").join("inbox");
    fs::create_dir_all(&inbox_path).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Mock scp: on pull, write fixed content to the destination.
    let on_pull = "        echo '{\"data\": \"test\"}' > \"$LAST\"\n        exit 0";
    write_mock_scp(&bin_dir, "        exit 1", on_pull);

    // Mock ssh: `ls` returns two files; `rm` exits 0.
    write_mock_ssh(
        &bin_dir,
        &[
            (
                "ls",
                "    echo 'msg-001.json'\n    echo 'msg-002.json'\n    exit 0",
            ),
            ("rm", "    exit 0"),
        ],
    );

    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );

    let result = with_mock_path(&bin_dir, || backend.poll_inbox(&inbox_path));

    let messages = result.expect("poll_inbox should succeed");
    assert_eq!(messages.len(), 2, "should return two messages");
}

/// `annotate_run` pushes an annotation file to the remote run directory.
#[test]
#[serial]
fn test_annotate_run_pushes_annotation_file() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let marker_file = tmp.path().join("push_called");
    let on_push = format!("        touch '{}'\n        exit 0", marker_file.display());
    write_mock_scp(&bin_dir, &on_push, "        exit 1");
    write_mock_ssh(&bin_dir, &[("mkdir", "    exit 0")]);

    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );

    let result = with_mock_path(&bin_dir, || {
        backend.annotate_run(&run_dir, "/some/manifest.toml")
    });

    result.expect("annotate_run should succeed");
    assert!(
        marker_file.exists(),
        "push_called marker should have been created"
    );
}

/// `save_checkpoint_summary` pushes a checkpoint file to the remote assay dir.
#[test]
#[serial]
fn test_save_checkpoint_summary_pushes_checkpoint() {
    use assay_types::TeamCheckpoint;

    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join("assay");
    fs::create_dir_all(&assay_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let marker_file = tmp.path().join("push_called");
    let on_push = format!("        touch '{}'\n        exit 0", marker_file.display());
    write_mock_scp(&bin_dir, &on_push, "        exit 1");
    write_mock_ssh(&bin_dir, &[("mkdir", "    exit 0")]);

    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );

    let checkpoint = TeamCheckpoint {
        version: 1,
        session_id: "test-session".to_string(),
        project: "/tmp/test-project".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        trigger: "manual".to_string(),
        agents: vec![],
        tasks: vec![],
        context_health: None,
    };

    let result = with_mock_path(&bin_dir, || {
        backend.save_checkpoint_summary(&assay_dir, &checkpoint)
    });

    result.expect("save_checkpoint_summary should succeed");
    assert!(
        marker_file.exists(),
        "push_called marker should have been created"
    );
}

// ── Test: Injection Safety ───────────────────────────────────────────────────

/// Paths with spaces in `remote_assay_dir` must be delivered as single tokens
/// to `scp`/`ssh` via `Command::arg()` — no shell word-splitting.
#[test]
#[serial]
fn test_injection_safety_path_with_spaces() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let arg_file = tmp.path().join("scp_args.log");

    // Mock scp: record all argv to the arg file, one arg per line.
    let on_push = format!(
        "        for arg in \"$@\"; do echo \"$arg\" >> '{}'; done\n        exit 0",
        arg_file.display()
    );
    write_mock_scp(&bin_dir, &on_push, "        exit 1");

    // Mock ssh: record argv for mkdir and exit 0.
    let ssh_arg_file = tmp.path().join("ssh_args.log");
    let mkdir_handler = format!(
        "    for arg in \"$@\"; do echo \"$arg\" >> '{}'; done\n    exit 0",
        ssh_arg_file.display()
    );
    write_mock_ssh(&bin_dir, &[("mkdir", &mkdir_handler)]);

    // Use a remote path with spaces — this must NOT be word-split by the shell.
    let backend = SshSyncBackend::new(
        "localhost".to_string(),
        "/remote/assay dir with spaces".to_string(),
        None,
        None,
        tmp.path().to_path_buf(),
    );
    let status = sample_status();

    // Set ARG_FILE env var for inspection (not used by mock, but available for debugging).
    unsafe { std::env::set_var("ARG_FILE", arg_file.to_str().unwrap()) };

    with_mock_path(&bin_dir, || {
        backend
            .push_session_event(&run_dir, &status)
            .expect("push_session_event should succeed with spaces in path");
    });

    unsafe { std::env::remove_var("ARG_FILE") };

    // Read the recorded scp args and verify the remote spec with spaces
    // appears as a single token (one line), not split across multiple lines.
    let args_content = fs::read_to_string(&arg_file).expect("scp_args.log should exist");

    // Find the arg that contains the remote path spec (host:path).
    let remote_args: Vec<&str> = args_content
        .lines()
        .filter(|line| line.contains("localhost:") && line.contains("assay dir with spaces"))
        .collect();

    assert!(
        !remote_args.is_empty(),
        "should have at least one arg containing the full remote path with spaces; args were:\n{}",
        args_content
    );

    // Each matching line should contain the FULL path — proving it was passed
    // as a single argument, not split on spaces.
    for arg in &remote_args {
        assert!(
            arg.contains("/remote/assay dir with spaces"),
            "remote path should appear as a single token with spaces preserved, got: {arg}"
        );
    }
}
