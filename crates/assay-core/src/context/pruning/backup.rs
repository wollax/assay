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
/// Creates `backup_dir` if it does not exist. Returns the path
/// to the created backup file.
pub fn backup_session(session_path: &Path, backup_dir: &Path) -> crate::Result<PathBuf> {
    std::fs::create_dir_all(backup_dir).map_err(|source| crate::AssayError::Io {
        operation: "creating backup directory".into(),
        path: backup_dir.to_path_buf(),
        source,
    })?;

    let session_name = session_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let backup_name = format!("{session_name}_{timestamp}.jsonl");
    let backup_path = backup_dir.join(&backup_name);

    std::fs::copy(session_path, &backup_path).map_err(|source| crate::AssayError::Io {
        operation: "backing up session file".into(),
        path: session_path.to_path_buf(),
        source,
    })?;

    prune_old_backups(backup_dir, &session_name, DEFAULT_RETENTION_LIMIT)?;

    Ok(backup_path)
}

/// List available backups for a specific session, sorted newest first.
///
/// Looks for files matching `{session_id}_*.jsonl` in `backup_dir`.
pub fn list_backups(backup_dir: &Path, session_id: &str) -> crate::Result<Vec<PathBuf>> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let prefix = format!("{session_id}_");
    let mut backups: Vec<PathBuf> = std::fs::read_dir(backup_dir)
        .map_err(|source| crate::AssayError::Io {
            operation: "listing backup directory".into(),
            path: backup_dir.to_path_buf(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".jsonl"))
        })
        .collect();

    // Sort by filename descending (newest first, since timestamps sort lexicographically)
    backups.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    Ok(backups)
}

/// Restore a backup to the session path.
///
/// Copies the backup file contents to the session file location.
pub fn restore_backup(backup_path: &Path, session_path: &Path) -> crate::Result<()> {
    std::fs::copy(backup_path, session_path).map_err(|source| crate::AssayError::Io {
        operation: "restoring backup".into(),
        path: backup_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

/// Remove old backups exceeding the retention limit.
///
/// Lists backups for `session_name`, sorted newest first, and deletes
/// any beyond `limit`. If fewer backups exist than `limit`, nothing
/// is deleted.
pub fn prune_old_backups(
    backup_dir: &Path,
    session_name: &str,
    limit: usize,
) -> crate::Result<()> {
    let backups = list_backups(backup_dir, session_name)?;

    for old_backup in backups.iter().skip(limit) {
        std::fs::remove_file(old_backup).map_err(|source| crate::AssayError::Io {
            operation: "pruning old backup".into(),
            path: old_backup.clone(),
            source,
        })?;
    }

    Ok(())
}
