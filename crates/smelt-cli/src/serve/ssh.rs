//! SSH client abstraction for dispatching commands to remote worker hosts.
//!
//! # Design
//!
//! - `SshClient` is a trait with async methods — not object-safe. Use generics
//!   (`impl SshClient` / `<C: SshClient>`) at call sites per D060.
//! - `SubprocessSshClient` shells out to the system `ssh` binary via
//!   `tokio::process::Command` to stay consistent with the async dispatch loop.
//! - Offline fast-fail is delegated to SSH's own `-o ConnectTimeout=<N>` flag so
//!   the subprocess self-terminates on timeout — no zombie processes (D111).
//! - `key_env` is an env-var *name*; the resolved path may appear in DEBUG logs
//!   but is never logged at INFO/WARN level (D112).

use std::path::PathBuf;

use anyhow::anyhow;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::serve::config::WorkerConfig;
use crate::serve::types::JobId;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Captured output from a remote SSH command.
#[derive(Debug, Clone)]
pub struct SshOutput {
    pub stdout: String,
    pub stderr: String,
    /// Exit code returned by the remote process.  Mapped to `-1` when the
    /// subprocess was killed by a signal and no numeric code is available.
    pub exit_code: i32,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Async SSH client abstraction.
///
/// # Object safety
///
/// This trait is intentionally **not** object-safe (it uses `async fn`).  Use
/// `impl SshClient` / `<C: SshClient>` at call sites rather than `dyn SshClient`.
#[allow(async_fn_in_trait)]
pub trait SshClient {
    /// Execute `cmd` on the remote host described by `worker`.
    ///
    /// Returns `SshOutput` on any successful subprocess invocation — callers
    /// should inspect `exit_code` to detect remote command failures.
    async fn exec(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        cmd: &str,
    ) -> anyhow::Result<SshOutput>;

    /// Verify connectivity to `worker` by running `echo smelt-probe`.
    ///
    /// Returns `Ok(())` when the probe succeeds (exit_code == 0), or `Err`
    /// otherwise.  The error is returned within `timeout_secs + 1s` thanks to
    /// SSH's own `ConnectTimeout` option.
    async fn probe(&self, worker: &WorkerConfig, timeout_secs: u64) -> anyhow::Result<()>;

    /// Copy a local file to a remote destination via `scp`.
    ///
    /// `remote_dest` is in `user@host:/path` format.
    async fn scp_to(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        local_path: &std::path::Path,
        remote_dest: &str,
    ) -> anyhow::Result<()>;

    /// Copy a remote file or directory to a local destination.
    ///
    /// `remote_src` is a path on the remote host (e.g. `/tmp/.smelt/runs/my-job/`).
    /// The method builds the `user@host:<remote_src>` spec internally.
    ///
    /// Note: `SubprocessSshClient` adds `-r` for recursive copy; other
    /// implementations may handle recursion differently.
    async fn scp_from(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        remote_src: &str,
        local_dest: &std::path::Path,
    ) -> anyhow::Result<()>;
}

// ---------------------------------------------------------------------------
// SubprocessSshClient
// ---------------------------------------------------------------------------

/// `SshClient` implementation that shells out to the system `ssh` binary.
pub struct SubprocessSshClient;

impl SubprocessSshClient {
    /// Resolve the path to the `ssh` binary using [`which::which`].
    fn ssh_binary() -> anyhow::Result<PathBuf> {
        which::which("ssh").map_err(|e| anyhow!("ssh binary not found in PATH: {}", e))
    }

    /// Build the argument list for an SSH invocation.
    ///
    /// Common flags:
    /// - `-o BatchMode=yes` — never prompt for a password
    /// - `-o StrictHostKeyChecking=accept-new` — add new keys; reject changed keys
    /// - `-o ConnectTimeout=<N>` — fast-fail for offline workers
    /// - `-p <port>` when `port != 22`
    /// - `-i <key_path>` when `key_env` resolves to a non-empty path; omitted
    ///   (with a WARN log) when the env var is unset
    ///
    /// `extra_args` are appended verbatim after the common flags (e.g.
    /// `["user@host", "echo hello"]`).
    pub fn build_ssh_args(
        worker: &WorkerConfig,
        timeout_secs: u64,
        extra_args: &[&str],
    ) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
            "-o".to_string(),
            format!("ConnectTimeout={timeout_secs}"),
        ];

        if worker.port != 22 {
            args.push("-p".to_string());
            args.push(worker.port.to_string());
        }

        match std::env::var(&worker.key_env) {
            Ok(key_path) if !key_path.is_empty() => {
                debug!(
                    key_path = %key_path,
                    key_env = %worker.key_env,
                    "using SSH identity file"
                );
                args.push("-i".to_string());
                args.push(key_path);
            }
            Ok(_) => {
                warn!(
                    key_env = %worker.key_env,
                    host = %worker.host,
                    "key_env is set but resolves to an empty path; SSH will use default keys"
                );
            }
            Err(_) => {
                warn!(
                    key_env = %worker.key_env,
                    host = %worker.host,
                    "key_env is not set; SSH will use default keys"
                );
            }
        }

        for arg in extra_args {
            args.push(arg.to_string());
        }

        args
    }

    /// Resolve the path to the `scp` binary using [`which::which`].
    fn scp_binary() -> anyhow::Result<PathBuf> {
        which::which("scp").map_err(|e| anyhow!("scp binary not found in PATH: {}", e))
    }

    /// Build the argument list for an SCP invocation.
    ///
    /// Mirrors [`build_ssh_args`] but uses uppercase `-P` for port (SCP
    /// convention) instead of lowercase `-p`.
    pub fn build_scp_args(
        worker: &WorkerConfig,
        timeout_secs: u64,
        extra_args: &[&str],
    ) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
            "-o".to_string(),
            format!("ConnectTimeout={timeout_secs}"),
        ];

        if worker.port != 22 {
            args.push("-P".to_string());
            args.push(worker.port.to_string());
        }

        match std::env::var(&worker.key_env) {
            Ok(key_path) if !key_path.is_empty() => {
                debug!(
                    key_path = %key_path,
                    key_env = %worker.key_env,
                    "using SCP identity file"
                );
                args.push("-i".to_string());
                args.push(key_path);
            }
            Ok(_) => {
                warn!(
                    key_env = %worker.key_env,
                    host = %worker.host,
                    "key_env is set but resolves to an empty path; SCP will use default keys"
                );
            }
            Err(_) => {
                warn!(
                    key_env = %worker.key_env,
                    host = %worker.host,
                    "key_env is not set; SCP will use default keys"
                );
            }
        }

        for arg in extra_args {
            args.push(arg.to_string());
        }

        args
    }
}

impl SshClient for SubprocessSshClient {
    async fn exec(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        cmd: &str,
    ) -> anyhow::Result<SshOutput> {
        let ssh = Self::ssh_binary()?;
        let target = format!("{}@{}", worker.user, worker.host);
        let extra: &[&str] = &[&target, cmd];
        let args = Self::build_ssh_args(worker, timeout_secs, extra);

        debug!(
            host = %worker.host,
            cmd = %cmd,
            args = ?args,
            "ssh exec entry"
        );

        let output = Command::new(&ssh)
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow!("failed to spawn ssh for host {}: {}", worker.host, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if exit_code != 0 {
            warn!(
                host = %worker.host,
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "ssh exec failed"
            );
        }

        Ok(SshOutput {
            stdout,
            stderr,
            exit_code,
        })
    }

    async fn probe(&self, worker: &WorkerConfig, timeout_secs: u64) -> anyhow::Result<()> {
        let result = self.exec(worker, timeout_secs, "echo smelt-probe").await;
        match result {
            Ok(out) if out.exit_code == 0 => Ok(()),
            Ok(out) => Err(anyhow!(
                "ssh probe failed for host {}: exit_code={} stderr={}",
                worker.host,
                out.exit_code,
                out.stderr.trim()
            )),
            Err(e) => Err(anyhow!("ssh probe failed for host {}: {}", worker.host, e)),
        }
    }

    async fn scp_to(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        local_path: &std::path::Path,
        remote_dest: &str,
    ) -> anyhow::Result<()> {
        let scp = Self::scp_binary()?;
        let local_str = local_path.to_string_lossy();
        let extra: &[&str] = &[&local_str, remote_dest];
        let args = Self::build_scp_args(worker, timeout_secs, extra);

        debug!(
            host = %worker.host,
            local_path = %local_path.display(),
            remote_dest = %remote_dest,
            "scp_to entry"
        );

        let output = Command::new(&scp)
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow!("failed to spawn scp for host {}: {}", worker.host, e))?;

        let exit_code = output.status.code().unwrap_or(-1);
        if exit_code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                host = %worker.host,
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "scp_to non-zero exit"
            );
            return Err(anyhow!(
                "scp to {} failed: exit_code={} stderr={}",
                worker.host,
                exit_code,
                stderr.trim()
            ));
        }

        Ok(())
    }

    async fn scp_from(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        remote_src: &str,
        local_dest: &std::path::Path,
    ) -> anyhow::Result<()> {
        let scp = Self::scp_binary()?;
        let remote_spec = format!("{}@{}:{}", worker.user, worker.host, remote_src);
        let local_str = local_dest.to_string_lossy();
        let extra: &[&str] = &["-r", &remote_spec, &local_str];
        let args = Self::build_scp_args(worker, timeout_secs, extra);

        debug!(
            host = %worker.host,
            remote_src = %remote_src,
            local_dest = %local_dest.display(),
            "scp_from entry"
        );

        let output = Command::new(&scp)
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow!("failed to spawn scp for host {}: {}", worker.host, e))?;

        let exit_code = match output.status.code() {
            Some(code) => code,
            None => {
                // Process killed by signal (Unix) — log the signal number.
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    let sig = output.status.signal();
                    warn!(
                        host = %worker.host,
                        signal = ?sig,
                        "scp_from killed by signal"
                    );
                }
                -1
            }
        };
        if exit_code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                host = %worker.host,
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "scp_from non-zero exit"
            );
            return Err(anyhow!(
                "scp from {} failed: exit_code={} stderr={}",
                worker.host,
                exit_code,
                stderr.trim()
            ));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// SCP the local manifest file to `/tmp/smelt-<job_id>.toml` on the worker.
///
/// Returns the remote path string on success.
pub async fn deliver_manifest<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    job_id: &JobId,
    local_manifest: &std::path::Path,
) -> anyhow::Result<String> {
    let remote_path = format!("/tmp/smelt-{}.toml", job_id);
    let remote_dest = format!("{}@{}:{}", worker.user, worker.host, remote_path);
    client
        .scp_to(worker, timeout_secs, local_manifest, &remote_dest)
        .await?;
    Ok(remote_path)
}

/// SCP the remote `.smelt/runs/<job_name>/` directory back to the local
/// filesystem so `smelt status` can read job state after remote execution.
///
/// `job_name` is the manifest's `[job] name` field (not the queue `JobId`) —
/// it must match what `smelt run` uses to compute its state directory.
///
/// Creates the local target directory (`local_dest_dir/.smelt/runs/<job_name>/`)
/// before calling `scp_from`.
pub async fn sync_state_back<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    job_name: &str,
    local_dest_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let remote_src = format!("/tmp/.smelt/runs/{}/", job_name);
    let local_target = local_dest_dir.join(".smelt/runs").join(job_name);

    debug!(
        host = %worker.host,
        job_name = %job_name,
        local_dest_dir = %local_dest_dir.display(),
        "sync_state_back entry"
    );

    std::fs::create_dir_all(&local_target).map_err(|e| {
        anyhow!(
            "failed to create local state dir {}: {}",
            local_target.display(),
            e
        )
    })?;

    client
        .scp_from(worker, timeout_secs, &remote_src, &local_target)
        .await
}

/// SSH exec `smelt run <remote_manifest_path>` on the worker, returning the
/// raw exit code.  Callers map 0/2/other per D050.
pub async fn run_remote_job<C: SshClient>(
    client: &C,
    worker: &WorkerConfig,
    timeout_secs: u64,
    remote_manifest_path: &str,
) -> anyhow::Result<i32> {
    let cmd = format!("smelt run {}", remote_manifest_path);
    let output = client.exec(worker, timeout_secs, &cmd).await?;
    if output.exit_code == 127 {
        warn!(
            host = %worker.host,
            cmd = %cmd,
            "exit code 127: smelt may not be on the remote PATH"
        );
    }
    Ok(output.exit_code)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::*;

    // -----------------------------------------------------------------------
    // MockSshClient
    // -----------------------------------------------------------------------

    /// Test double for `SshClient` with configurable pop-front results.
    pub(crate) struct MockSshClient {
        exec_results: Arc<Mutex<VecDeque<anyhow::Result<SshOutput>>>>,
        probe_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
        scp_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
        scp_from_results: Arc<Mutex<VecDeque<anyhow::Result<()>>>>,
    }

    impl MockSshClient {
        pub fn new() -> Self {
            Self {
                exec_results: Arc::new(Mutex::new(VecDeque::new())),
                probe_results: Arc::new(Mutex::new(VecDeque::new())),
                scp_results: Arc::new(Mutex::new(VecDeque::new())),
                scp_from_results: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        pub fn with_exec_result(self, result: anyhow::Result<SshOutput>) -> Self {
            self.exec_results.lock().unwrap().push_back(result);
            self
        }

        pub fn with_scp_result(self, result: anyhow::Result<()>) -> Self {
            self.scp_results.lock().unwrap().push_back(result);
            self
        }

        #[allow(dead_code)]
        pub fn with_probe_result(self, result: anyhow::Result<()>) -> Self {
            self.probe_results.lock().unwrap().push_back(result);
            self
        }

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

        async fn probe(
            &self,
            _worker: &WorkerConfig,
            _timeout_secs: u64,
        ) -> anyhow::Result<()> {
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
            args_str
                .windows(2)
                .any(|w| w == ["-o", "ConnectTimeout=3"]),
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
        let args = SubprocessSshClient::build_scp_args(&worker, 5, &["/local/file", "user@host:/remote"]);
        let args_str: Vec<&str> = args.iter().map(String::as_str).collect();

        assert!(
            args_str.windows(2).any(|w| w == ["-o", "BatchMode=yes"]),
            "expected BatchMode=yes in scp args: {args_str:?}"
        );
        assert!(
            args_str
                .windows(2)
                .any(|w| w == ["-o", "ConnectTimeout=5"]),
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
        assert!(result.is_ok(), "deliver_manifest should succeed: {:?}", result.err());
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
        let args = SubprocessSshClient::build_scp_args(
            &worker,
            5,
            &["-r", remote_spec, local_dest],
        );
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
        let client = MockSshClient::new()
            .with_scp_from_result(Err(anyhow!("scp failed: connection refused")));
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
        let client =
            MockSshClient::new().with_scp_from_result(Err(anyhow!("scp failed")));
        let worker = test_worker();
        let tmp = tempfile::TempDir::new().unwrap();

        let result = sync_state_back(&client, &worker, 5, "test-job", tmp.path()).await;
        assert!(result.is_err(), "sync_state_back should propagate scp_from error");
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
            output.exit_code, 0,
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
}
