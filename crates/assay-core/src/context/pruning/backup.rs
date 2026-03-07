//! Backup and restore operations for session pruning.
//!
//! Creates timestamped backups before destructive operations and provides
//! restore and retention management.

use std::path::{Path, PathBuf};

/// Default number of backups to retain per session.
pub const DEFAULT_RETENTION_LIMIT: usize = 5;

/// Create a backup of the session file in `backup_dir`.
///
/// The backup filename includes a UTC timestamp for ordering.
/// Creates `backup_dir` if it does not exist.
pub fn backup_session(_session_path: &Path, _backup_dir: &Path) -> crate::Result<PathBuf> {
    todo!("Implemented in Task 2")
}

/// List available backups for a specific session, newest first.
pub fn list_backups(_backup_dir: &Path, _session_id: &str) -> crate::Result<Vec<PathBuf>> {
    todo!("Implemented in Task 2")
}

/// Restore a backup to the session path.
pub fn restore_backup(_backup_path: &Path, _session_path: &Path) -> crate::Result<()> {
    todo!("Implemented in Task 2")
}

/// Remove old backups exceeding the retention limit.
pub fn prune_old_backups(
    _backup_dir: &Path,
    _session_name: &str,
    _limit: usize,
) -> crate::Result<()> {
    todo!("Implemented in Task 2")
}
