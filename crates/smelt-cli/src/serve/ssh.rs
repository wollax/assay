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
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

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
