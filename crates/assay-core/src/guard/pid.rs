//! PID file management for single-instance guard daemon.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::AssayError;

/// Default PID file location within `.assay/guard/`.
pub fn pid_file_path(assay_dir: &Path) -> PathBuf {
    assay_dir.join("guard").join("guard.pid")
}

/// Check if a guard daemon is already running.
///
/// Returns `Some(pid)` if the process is alive, `None` if no PID file exists
/// or the process is dead (stale PID file is cleaned up automatically).
pub fn check_running(pid_path: &Path) -> Option<u32> {
    let contents = match fs::read_to_string(pid_path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let pid: u32 = match contents.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            // Corrupt PID file — remove it
            let _ = fs::remove_file(pid_path);
            return None;
        }
    };

    if is_process_alive(pid) {
        Some(pid)
    } else {
        // Stale PID file — process is dead, clean up
        let _ = fs::remove_file(pid_path);
        None
    }
}

/// Create a PID file with the current process ID.
///
/// Fails with [`AssayError::GuardAlreadyRunning`] if a live process holds the PID file.
pub fn create_pid_file(pid_path: &Path) -> crate::Result<()> {
    if let Some(pid) = check_running(pid_path) {
        return Err(AssayError::GuardAlreadyRunning { pid });
    }

    // Ensure parent directory exists
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent).map_err(|source| AssayError::Io {
            operation: "creating guard PID directory".into(),
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let mut file = fs::File::create(pid_path).map_err(|source| AssayError::Io {
        operation: "creating guard PID file".into(),
        path: pid_path.to_path_buf(),
        source,
    })?;

    file.write_all(std::process::id().to_string().as_bytes())
        .map_err(|source| AssayError::Io {
            operation: "writing guard PID file".into(),
            path: pid_path.to_path_buf(),
            source,
        })?;

    file.sync_all().map_err(|source| AssayError::Io {
        operation: "syncing guard PID file".into(),
        path: pid_path.to_path_buf(),
        source,
    })
}

/// Remove the PID file (called on shutdown).
///
/// Ignores `NotFound` errors (file may already be gone).
pub fn remove_pid_file(pid_path: &Path) -> crate::Result<()> {
    match fs::remove_file(pid_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(AssayError::Io {
            operation: "removing guard PID file".into(),
            path: pid_path.to_path_buf(),
            source,
        }),
    }
}

/// Check whether a process with the given PID is alive.
#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    // Guard against PIDs that would produce invalid values when cast to i32:
    // - PID 0 means "all processes in the current process group" for kill()
    // - PIDs > i32::MAX wrap to negative values, with -1 meaning "all processes"
    let Ok(pid_i32) = i32::try_from(pid) else {
        return false;
    };
    if pid_i32 <= 0 {
        return false;
    }
    // SAFETY: kill(pid, 0) is a standard POSIX operation that checks process existence
    // without sending a signal. Returns 0 if process exists, -1 with ESRCH if not.
    unsafe { libc::kill(pid_i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    // On non-Unix platforms, assume the process is not alive.
    // Guard daemon is Unix-only for now.
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let pid_path = dir.path().join("guard").join("guard.pid");
        (dir, pid_path)
    }

    #[test]
    fn create_and_check_running_finds_current_process() {
        let (_dir, pid_path) = setup();

        create_pid_file(&pid_path).unwrap();
        let running = check_running(&pid_path);
        assert_eq!(running, Some(std::process::id()));
    }

    #[test]
    fn check_running_returns_none_when_no_file() {
        let (_dir, pid_path) = setup();
        assert_eq!(check_running(&pid_path), None);
    }

    #[test]
    fn remove_pid_file_then_check_returns_none() {
        let (_dir, pid_path) = setup();

        create_pid_file(&pid_path).unwrap();
        remove_pid_file(&pid_path).unwrap();
        assert_eq!(check_running(&pid_path), None);
    }

    #[test]
    fn remove_nonexistent_pid_file_is_ok() {
        let (_dir, pid_path) = setup();
        assert!(remove_pid_file(&pid_path).is_ok());
    }

    #[test]
    fn stale_pid_is_cleaned_up() {
        let (_dir, pid_path) = setup();

        // Write a PID that almost certainly doesn't exist
        fs::create_dir_all(pid_path.parent().unwrap()).unwrap();
        fs::write(&pid_path, "4294967295").unwrap(); // max u32, unlikely to be alive

        let running = check_running(&pid_path);
        assert_eq!(running, None, "stale PID should return None");
        assert!(!pid_path.exists(), "stale PID file should be cleaned up");
    }

    #[test]
    fn double_create_returns_guard_already_running() {
        let (_dir, pid_path) = setup();

        create_pid_file(&pid_path).unwrap();
        let err = create_pid_file(&pid_path).unwrap_err();

        match err {
            AssayError::GuardAlreadyRunning { pid } => {
                assert_eq!(pid, std::process::id());
            }
            other => panic!("expected GuardAlreadyRunning, got: {other}"),
        }
    }

    #[test]
    fn corrupt_pid_file_is_cleaned_up() {
        let (_dir, pid_path) = setup();

        fs::create_dir_all(pid_path.parent().unwrap()).unwrap();
        fs::write(&pid_path, "not-a-number").unwrap();

        assert_eq!(check_running(&pid_path), None);
        assert!(!pid_path.exists(), "corrupt PID file should be removed");
    }

    #[test]
    fn pid_file_path_default() {
        let path = pid_file_path(Path::new("/home/user/.assay"));
        assert_eq!(path, PathBuf::from("/home/user/.assay/guard/guard.pid"));
    }
}
