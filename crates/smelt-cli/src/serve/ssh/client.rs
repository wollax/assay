//! `SubprocessSshClient` — shells out to the system `ssh`/`scp` binaries.

use std::path::PathBuf;

use anyhow::anyhow;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::serve::config::WorkerConfig;

use super::{SshClient, SshOutput};

// ---------------------------------------------------------------------------
// SubprocessSshClient
// ---------------------------------------------------------------------------

/// `SshClient` implementation that shells out to the system `ssh` binary.
#[derive(Clone)]
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
    /// Mirrors `build_ssh_args` but uses uppercase `-P` for port (SCP
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
