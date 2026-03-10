//! Process group management for agent sessions.
//!
//! This module provides infrastructure for isolating spawned processes
//! into process groups for clean shutdown. Used by real-agent sessions
//! (Phase 5+), not by scripted sessions (which run in-process).

use std::process::Child;

/// A process group handle for clean shutdown of spawned processes.
///
/// Wraps a `Child` process that was spawned with `process_group(0)`,
/// ensuring the entire process group can be signaled for cleanup.
#[derive(Debug)]
pub struct ProcessGroup {
    child: Option<Child>,
    pgid: Option<u32>,
}

impl ProcessGroup {
    /// Create a new ProcessGroup wrapping a spawned child.
    ///
    /// The child MUST have been spawned with `.process_group(0)` to create
    /// a new process group.
    pub fn new(child: Child) -> Self {
        let pgid = child.id();
        Self {
            child: Some(child),
            pgid: Some(pgid),
        }
    }

    /// Send SIGTERM to the entire process group.
    ///
    /// Returns Ok(()) if the signal was sent, or if the process already exited.
    #[cfg(unix)]
    pub fn kill_group(&self) -> std::io::Result<()> {
        if let Some(pgid) = self.pgid {
            // Negate PID to signal the entire process group.
            // The child was spawned with process_group(0), so PID == PGID.
            let pid = i32::try_from(pgid).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "PID exceeds i32::MAX")
            })?;
            let ret = unsafe { libc::kill(-pid, libc::SIGTERM) };
            if ret == -1 {
                let err = std::io::Error::last_os_error();
                // ESRCH = process not found (already dead) — that's OK
                if err.raw_os_error() != Some(libc::ESRCH) {
                    return Err(err);
                }
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Wait for the child process to exit, returning its exit status.
    ///
    /// Returns an error if the child has already been waited on.
    pub fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child
            .as_mut()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "ProcessGroup::wait called after child was already waited on",
                )
            })?
            .wait()
    }
}
