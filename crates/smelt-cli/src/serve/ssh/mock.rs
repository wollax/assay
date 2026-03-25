//! Test double and unit tests for the SSH client abstraction.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::anyhow;

use crate::serve::config::WorkerConfig;

use super::client::SubprocessSshClient;
use super::operations::{deliver_manifest, run_remote_job, sync_state_back};
use super::{SshClient, SshOutput};
use crate::serve::types::JobId;

// -----------------------------------------------------------------------
// MockSshClient
// -----------------------------------------------------------------------

/// Test double for `SshClient` with configurable pop-front results.
#[derive(Clone)]
pub(crate) struct MockSshClient {
    exec_results: Arc<Mutex<VecDeque<anyhow::Result<SshOutput>>>>,
    probe_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
    scp_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
    scp_from_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
}

impl MockSshClient {
    /// Create a new `MockSshClient` with empty result queues.
    pub fn new() -> Self {
        Self {
            exec_results: Arc::new(Mutex::new(VecDeque::new())),
            probe_results: Arc::new(Mutex::new(VecDeque::new())),
            scp_results: Arc::new(Mutex::new(VecDeque::new())),
            scp_from_results: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Enqueue an exec result.
    pub fn with_exec_result(self, result: anyhow::Result<SshOutput>) -> Self {
        self.exec_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue an scp_to result.
    pub fn with_scp_result(self, result: anyhow::Result<()>) -> Self {
        self.scp_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue a probe result.
    pub fn with_probe_result(self, result: anyhow::Result<()>) -> Self {
        self.probe_results.lock().unwrap().push_back(result);
        self
    }

    /// Enqueue an scp_from result.
    pub fn with_scp_from_result(self, result: anyhow::Result<()>) -> Self {
        self.scp_from_results.lock().unwrap().push_back(result);
        self
    }
}

impl SshClient for MockSshClient {
    async fn exec(
        &self,
        _worker: &WorkerConfig,
        _timeout_secs: u64,
        _cmd: &str,
    ) -> anyhow::Result<SshOutput> {
        self.exec_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("MockSshClient: no exec results configured")))
    }

    async fn probe(&self, _worker: &WorkerConfig, _timeout_secs: u64) -> anyhow::Result<()> {
        self.probe_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("MockSshClient: no probe results configured")))
    }

    async fn scp_to(
        &self,
        _worker: &WorkerConfig,
        _timeout_secs: u64,
        _local_path: &std::path::Path,
        _remote_dest: &str,
    ) -> anyhow::Result<()> {
        self.scp_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("MockSshClient: no scp results configured")))
    }

    async fn scp_from(
        &self,
        _worker: &WorkerConfig,
        _timeout_secs: u64,
        _remote_src: &str,
        _local_dest: &std::path::Path,
    ) -> anyhow::Result<()> {
        self.scp_from_results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("MockSshClient: no scp_from results configured")))
    }
}

/// Helper: build a test WorkerConfig.
fn test_worker() -> WorkerConfig {
    WorkerConfig {
        host: "testhost".to_string(),
        user: "testuser".to_string(),
        key_env: "SMELT_SSH_KEY_NONEXISTENT_XYZ".to_string(),
        port: 22,
    }
}

// -----------------------------------------------------------------------
// build_ssh_args unit tests
// -----------------------------------------------------------------------

/// Non-gated unit test: verifies that `build_ssh_args` assembles the expected
/// flag set without making any actual SSH connection.
///
/// Runs in `cargo test --workspace` without `SMELT_SSH_TEST=1`.
#[test]
fn test_ssh_args_build() {
    let worker = WorkerConfig {
        host: "myhost".to_string(),
        user: "alice".to_string(),
        key_env: "SMELT_SSH_KEY_NONEXISTENT_XYZ".to_string(),
        port: 22,
    };

    let args = SubprocessSshClient::build_ssh_args(&worker, 3, &["alice@myhost", "echo hi"]);
    let args_str: Vec<&str> = args.iter().map(String::as_str).collect();

    // Common required flags
    assert!(
        args_str.windows(2).any(|w| w == ["-o", "BatchMode=yes"]),
        "expected BatchMode=yes in args: {args_str:?}"
    );
    assert!(
        args_str
            .windows(2)
            .any(|w| w == ["-o", "StrictHostKeyChecking=accept-new"]),
        "expected StrictHostKeyChecking=accept-new in args: {args_str:?}"
    );
    assert!(
        args_str.windows(2).any(|w| w == ["-o", "ConnectTimeout=3"]),
        "expected ConnectTimeout=3 in args: {args_str:?}"
    );

    // Port 22 should NOT add -p flag
    assert!(
        !args_str.contains(&"-p"),
        "port 22 should not emit -p flag: {args_str:?}"
    );

    // user@host target present
    assert!(
        args_str.contains(&"alice@myhost"),
        "expected user@host in args: {args_str:?}"
    );
}

/// Verifies that a non-default port emits `-p <port>` in the args.
#[test]
fn test_ssh_args_build_custom_port() {
    let worker = WorkerConfig {
        host: "remote".to_string(),
        user: "bob".to_string(),
        key_env: "SMELT_SSH_KEY_NONEXISTENT_XYZ".to_string(),
        port: 2222,
    };

    let args = SubprocessSshClient::build_ssh_args(&worker, 10, &[]);
    let args_str: Vec<&str> = args.iter().map(String::as_str).collect();

    assert!(
        args_str.windows(2).any(|w| w == ["-p", "2222"]),
        "expected -p 2222 in args: {args_str:?}"
    );
}

// -----------------------------------------------------------------------
// SCP args unit tests
// -----------------------------------------------------------------------

#[test]
fn test_scp_args_build() {
    let worker = test_worker();
    let args =
        SubprocessSshClient::build_scp_args(&worker, 5, &["/local/file", "user@host:/remote"]);
    let args_str: Vec<&str> = args.iter().map(String::as_str).collect();

    assert!(
        args_str.windows(2).any(|w| w == ["-o", "BatchMode=yes"]),
        "expected BatchMode=yes in scp args: {args_str:?}"
    );
    assert!(
        args_str.windows(2).any(|w| w == ["-o", "ConnectTimeout=5"]),
        "expected ConnectTimeout=5 in scp args: {args_str:?}"
    );
    // Port 22 should NOT add -P flag
    assert!(
        !args_str.contains(&"-P"),
        "port 22 should not emit -P flag: {args_str:?}"
    );
    assert!(args_str.contains(&"/local/file"));
    assert!(args_str.contains(&"user@host:/remote"));
}

#[test]
fn test_scp_args_custom_port() {
    let worker = WorkerConfig {
        host: "remote".to_string(),
        user: "bob".to_string(),
        key_env: "SMELT_SSH_KEY_NONEXISTENT_XYZ".to_string(),
        port: 2222,
    };

    let args = SubprocessSshClient::build_scp_args(&worker, 10, &[]);
    let args_str: Vec<&str> = args.iter().map(String::as_str).collect();

    assert!(
        args_str.windows(2).any(|w| w == ["-P", "2222"]),
        "expected uppercase -P 2222 in scp args: {args_str:?}"
    );
    // Must NOT contain lowercase -p
    assert!(
        !args_str.contains(&"-p"),
        "scp should use -P (uppercase), not -p: {args_str:?}"
    );
}

// -----------------------------------------------------------------------
// Mock-based unit tests for deliver_manifest and run_remote_job
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_deliver_manifest_mock() {
    let client = MockSshClient::new().with_scp_result(Ok(()));
    let worker = test_worker();
    let job_id = JobId::new("job-1");
    let local = std::path::Path::new("/tmp/test-manifest.toml");

    let result = deliver_manifest(&client, &worker, 5, &job_id, local).await;
    assert!(
        result.is_ok(),
        "deliver_manifest should succeed: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), "/tmp/smelt-job-1.toml");
}

#[tokio::test]
async fn test_run_remote_job_mock_success() {
    let client = MockSshClient::new().with_exec_result(Ok(SshOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 0,
    }));
    let worker = test_worker();

    let result = run_remote_job(&client, &worker, 10, "/tmp/smelt-job-1.toml").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn test_run_remote_job_mock_exit2() {
    let client = MockSshClient::new().with_exec_result(Ok(SshOutput {
        stdout: String::new(),
        stderr: "some error".to_string(),
        exit_code: 2,
    }));
    let worker = test_worker();

    let result = run_remote_job(&client, &worker, 10, "/tmp/smelt-job-1.toml").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
}

// -----------------------------------------------------------------------
// scp_from unit tests
// -----------------------------------------------------------------------

/// Verifies that build_scp_args with `-r` produces the expected args:
/// - `-r` flag is present
/// - Remote spec comes before local path
/// - Uses uppercase `-P` for custom port (not lowercase `-p`)
#[test]
fn test_scp_from_args_recursive() {
    let worker = WorkerConfig {
        host: "remote".to_string(),
        user: "bob".to_string(),
        key_env: "SMELT_SSH_KEY_NONEXISTENT_XYZ".to_string(),
        port: 2222,
    };
    let remote_spec = "bob@remote:/home/bob/.smelt/runs/job-1";
    let local_dest = "/tmp/local-state";
    let args = SubprocessSshClient::build_scp_args(&worker, 5, &["-r", remote_spec, local_dest]);
    let args_str: Vec<&str> = args.iter().map(String::as_str).collect();

    // -r flag present
    assert!(
        args_str.contains(&"-r"),
        "expected -r flag in scp_from args: {args_str:?}"
    );

    // Remote spec comes before local path in the extra_args portion
    let r_pos = args_str.iter().position(|&a| a == remote_spec).unwrap();
    let l_pos = args_str.iter().position(|&a| a == local_dest).unwrap();
    assert!(
        r_pos < l_pos,
        "remote spec should come before local dest: remote@{r_pos}, local@{l_pos}"
    );

    // Uppercase -P for port, not lowercase -p
    assert!(
        args_str.windows(2).any(|w| w == ["-P", "2222"]),
        "expected uppercase -P 2222 in scp args: {args_str:?}"
    );
    assert!(
        !args_str.contains(&"-p"),
        "scp should use -P (uppercase), not -p: {args_str:?}"
    );
}

#[tokio::test]
async fn test_scp_from_mock_success() {
    let client = MockSshClient::new().with_scp_from_result(Ok(()));
    let worker = test_worker();
    let local_dest = std::path::Path::new("/tmp/local-state");

    let result = client
        .scp_from(&worker, 5, "/remote/.smelt/runs/job-1", local_dest)
        .await;
    assert!(
        result.is_ok(),
        "scp_from mock success should return Ok: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_scp_from_mock_failure() {
    let client =
        MockSshClient::new().with_scp_from_result(Err(anyhow!("scp failed: connection refused")));
    let worker = test_worker();
    let local_dest = std::path::Path::new("/tmp/local-state");

    let result = client
        .scp_from(&worker, 5, "/remote/.smelt/runs/job-1", local_dest)
        .await;
    assert!(result.is_err(), "scp_from mock failure should return Err");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("connection refused"),
        "error should contain original message: {err_msg}"
    );
}

// -----------------------------------------------------------------------
// sync_state_back unit tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_sync_state_back_mock_success() {
    let client = MockSshClient::new().with_scp_from_result(Ok(()));
    let worker = test_worker();
    let tmp = tempfile::TempDir::new().unwrap();

    let result = sync_state_back(&client, &worker, 5, "test-job", tmp.path()).await;
    assert!(
        result.is_ok(),
        "sync_state_back should succeed: {:?}",
        result.err()
    );

    // Verify local directory was created
    let expected_dir = tmp.path().join(".smelt/runs/test-job");
    assert!(
        expected_dir.exists(),
        "expected local dir to exist: {}",
        expected_dir.display()
    );
    assert!(
        expected_dir.is_dir(),
        "expected local path to be a directory: {}",
        expected_dir.display()
    );
}

#[tokio::test]
async fn test_sync_state_back_mock_failure() {
    let client = MockSshClient::new().with_scp_from_result(Err(anyhow!("scp failed")));
    let worker = test_worker();
    let tmp = tempfile::TempDir::new().unwrap();

    let result = sync_state_back(&client, &worker, 5, "test-job", tmp.path()).await;
    assert!(
        result.is_err(),
        "sync_state_back should propagate scp_from error"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("scp failed"),
        "error should contain original message: {err_msg}"
    );
}

// -----------------------------------------------------------------------
// Gated integration tests — require SMELT_SSH_TEST=1 and a running sshd
// on localhost.  Use: SMELT_SSH_TEST=1 cargo test -p smelt-cli --
// --include-ignored test_ssh
// -----------------------------------------------------------------------

/// Connect to localhost SSH, execute `echo hello`, assert stdout == "hello".
///
/// Requires: `sshd` running on localhost port 22, current user can auth via
/// default key or a key pointed to by `SMELT_SSH_KEY` env var.
#[tokio::test]
#[ignore]
async fn test_ssh_exec_localhost() {
    if std::env::var("SMELT_SSH_TEST").is_err() {
        return;
    }

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "root".to_string());

    let worker = WorkerConfig {
        host: "127.0.0.1".to_string(),
        user,
        key_env: "SMELT_SSH_KEY".to_string(),
        port: 22,
    };

    let client = SubprocessSshClient;
    let output = client
        .exec(&worker, 5, "echo hello")
        .await
        .expect("exec should not return an error");

    assert_eq!(
        output.exit_code,
        0,
        "exit_code should be 0, got {} (stderr: {})",
        output.exit_code,
        output.stderr.trim()
    );
    assert_eq!(
        output.stdout.trim(),
        "hello",
        "expected stdout 'hello', got {:?}",
        output.stdout
    );
}

/// Connect to a port that is almost certainly refused, verify Err is returned
/// within 4 seconds (SSH's ConnectTimeout=3 + 1s margin).
#[tokio::test]
#[ignore]
async fn test_ssh_probe_offline() {
    if std::env::var("SMELT_SSH_TEST").is_err() {
        return;
    }

    let worker = WorkerConfig {
        host: "127.0.0.1".to_string(),
        user: "nobody".to_string(),
        key_env: "SMELT_SSH_KEY_UNUSED".to_string(),
        port: 19222,
    };

    let client = SubprocessSshClient;
    let start = std::time::Instant::now();
    let result = client.probe(&worker, 3).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "probe to refused port should return Err");
    assert!(
        elapsed < Duration::from_secs(4),
        "probe should return within 4s, took {:?}",
        elapsed
    );
}
