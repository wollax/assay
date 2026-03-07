//! Guard daemon: background context protection with threshold-based pruning.

use std::path::Path;

use assay_types::GuardConfig;

pub mod circuit_breaker;
pub mod config;
pub mod daemon;
pub mod pid;
pub mod thresholds;
pub mod watcher;

/// Start the guard daemon. Blocks until shutdown or circuit breaker trip.
#[cfg(unix)]
pub async fn start_guard(
    session_path: &Path,
    assay_dir: &Path,
    config: GuardConfig,
) -> crate::Result<()> {
    let mut d =
        daemon::GuardDaemon::new(session_path.to_path_buf(), assay_dir.to_path_buf(), config);
    d.run().await
}

/// Stop a running guard daemon by reading its PID file and sending SIGTERM.
#[cfg(unix)]
pub fn stop_guard(assay_dir: &Path) -> crate::Result<()> {
    let pid_path = pid::pid_file_path(assay_dir);
    match pid::check_running(&pid_path) {
        Some(pid) => {
            let pid_i32 = i32::try_from(pid).map_err(|_| crate::AssayError::GuardNotRunning)?;
            // SAFETY: kill(pid, SIGTERM) is a standard POSIX operation to request
            // graceful termination of a process.
            unsafe {
                libc::kill(pid_i32, libc::SIGTERM);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
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
