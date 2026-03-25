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

/// Subprocess-based SSH/SCP client implementation.
pub mod client;
/// Free functions: manifest delivery, state sync, remote job execution.
pub mod operations;

#[cfg(test)]
pub(crate) mod mock;

// Re-export public API so existing `crate::serve::ssh::*` paths keep working.
pub use client::SubprocessSshClient;
pub use operations::{deliver_manifest, run_remote_job, sync_state_back};

use crate::serve::config::WorkerConfig;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Captured output from a remote SSH command.
#[derive(Debug, Clone)]
pub struct SshOutput {
    /// Standard output captured from the remote command.
    pub stdout: String,
    /// Standard error captured from the remote command.
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
///
/// All async methods return `Send` futures so they can be spawned on the tokio
/// runtime via `tokio::spawn`.
pub trait SshClient {
    /// Execute `cmd` on the remote host described by `worker`.
    ///
    /// Returns `SshOutput` on any successful subprocess invocation — callers
    /// should inspect `exit_code` to detect remote command failures.
    fn exec(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        cmd: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<SshOutput>> + Send;

    /// Verify connectivity to `worker` by running `echo smelt-probe`.
    ///
    /// Returns `Ok(())` when the probe succeeds (exit_code == 0), or `Err`
    /// otherwise.  The error is returned within `timeout_secs + 1s` thanks to
    /// SSH's own `ConnectTimeout` option.
    fn probe(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;

    /// Copy a local file to a remote destination via `scp`.
    ///
    /// `remote_dest` is in `user@host:/path` format.
    fn scp_to(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        local_path: &std::path::Path,
        remote_dest: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;

    /// Copy a remote file or directory to a local destination.
    ///
    /// `remote_src` is a path on the remote host (e.g. `/tmp/.smelt/runs/my-job/`).
    /// The method builds the `user@host:<remote_src>` spec internally.
    ///
    /// Note: `SubprocessSshClient` adds `-r` for recursive copy; other
    /// implementations may handle recursion differently.
    fn scp_from(
        &self,
        worker: &WorkerConfig,
        timeout_secs: u64,
        remote_src: &str,
        local_dest: &std::path::Path,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

// Compatibility shim — preserves the `crate::serve::ssh::tests::MockSshClient`
// import path used by dispatch.rs and serve/tests/. Can be removed if those
// consumers update their imports to `crate::serve::ssh::mock::MockSshClient`.
#[cfg(test)]
pub(crate) mod tests {
    pub(crate) use super::mock::MockSshClient;
}
