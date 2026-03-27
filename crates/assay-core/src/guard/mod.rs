//! Guard daemon: background context protection with threshold-based pruning.

use std::path::Path;

use assay_types::GuardConfig;

#[cfg(feature = "orchestrate")]
use crate::state_backend::StateBackend;
#[cfg(feature = "orchestrate")]
use std::sync::Arc;

pub mod circuit_breaker;
pub mod config;
pub mod daemon;
pub mod pid;
pub mod thresholds;
pub mod watcher;

/// Start the guard daemon. Blocks until shutdown or circuit breaker trip.
#[cfg(all(unix, feature = "orchestrate"))]
pub async fn start_guard(
    session_path: &Path,
    assay_dir: &Path,
    project_dir: &Path,
    config: GuardConfig,
    backend: Arc<dyn StateBackend>,
) -> crate::Result<()> {
    let mut d = daemon::GuardDaemon::new(
        session_path.to_path_buf(),
        assay_dir.to_path_buf(),
        project_dir.to_path_buf(),
        config,
        backend,
    );
    d.run().await
}

/// Start the guard daemon. Blocks until shutdown or circuit breaker trip.
#[cfg(all(unix, not(feature = "orchestrate")))]
pub async fn start_guard(
    session_path: &Path,
    assay_dir: &Path,
    project_dir: &Path,
    config: GuardConfig,
) -> crate::Result<()> {
    let mut d = daemon::GuardDaemon::new(
        session_path.to_path_buf(),
        assay_dir.to_path_buf(),
        project_dir.to_path_buf(),
        config,
    );
    d.run().await
}

/// Stop a running guard daemon by reading its PID file and sending SIGTERM.
///
/// Polls `check_running` with a timeout rather than unconditionally removing
/// the PID file, so the daemon's own graceful shutdown can clean up properly.
#[cfg(unix)]
pub fn stop_guard(assay_dir: &Path) -> crate::Result<()> {
    let pid_path = pid::pid_file_path(assay_dir);
    match pid::check_running(&pid_path) {
        Some(pid) => {
            let pid_i32 = i32::try_from(pid).map_err(|_| crate::AssayError::GuardNotRunning)?;
            // SAFETY: kill(pid, SIGTERM) is a standard POSIX operation to request
            // graceful termination of a process.
            let ret = unsafe { libc::kill(pid_i32, libc::SIGTERM) };
            if ret != 0 {
                let err = std::io::Error::last_os_error();
                // ESRCH means process already gone — clean up PID file
                if err.raw_os_error() == Some(libc::ESRCH) {
                    let _ = pid::remove_pid_file(&pid_path);
                    return Ok(());
                }
                return Err(crate::AssayError::Io {
                    operation: "sending SIGTERM to guard daemon".into(),
                    path: pid_path,
                    source: err,
                });
            }

            // Poll for process exit with timeout (up to 3 seconds)
            for _ in 0..6 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if pid::check_running(&pid_path).is_none() {
                    return Ok(());
                }
            }

            // Process didn't exit in time — clean up stale PID file
            let _ = pid::remove_pid_file(&pid_path);
            Ok(())
        }
        None => Err(crate::AssayError::GuardNotRunning),
    }
}

/// Check guard daemon status.
pub fn guard_status(assay_dir: &Path) -> GuardStatus {
    let pid_path = pid::pid_file_path(assay_dir);
    match pid::check_running(&pid_path) {
        Some(pid) => GuardStatus::Running { pid },
        None => GuardStatus::Stopped,
    }
}

/// Guard daemon status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardStatus {
    /// Daemon is running with the given PID.
    Running {
        /// The PID of the running guard process.
        pid: u32,
    },
    /// Daemon is not running.
    Stopped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_status_derives_debug() {
        let status = GuardStatus::Running { pid: 1234 };
        let debug = format!("{status:?}");
        assert!(debug.contains("1234"));

        let stopped = GuardStatus::Stopped;
        let debug = format!("{stopped:?}");
        assert!(debug.contains("Stopped"));
    }

    #[test]
    fn stop_guard_with_no_pid_returns_guard_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let result = stop_guard(dir.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::AssayError::GuardNotRunning => {}
            other => panic!("expected GuardNotRunning, got: {other}"),
        }
    }

    #[test]
    fn guard_status_with_no_pid_returns_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let status = guard_status(dir.path());
        assert_eq!(status, GuardStatus::Stopped);
    }
}
