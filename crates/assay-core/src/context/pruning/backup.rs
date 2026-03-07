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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_session_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "{content}").unwrap();
        path
    }

    fn create_fake_backup(backup_dir: &Path, session_id: &str, timestamp: &str) -> PathBuf {
        std::fs::create_dir_all(backup_dir).unwrap();
        let name = format!("{session_id}_{timestamp}.jsonl");
        let path = backup_dir.join(&name);
        std::fs::write(&path, "backup content").unwrap();
        path
    }

    #[test]
    fn backup_session_creates_copy_in_backup_dir() {
        let dir = tempfile::tempdir().unwrap();
        let session = create_session_file(dir.path(), "sess1.jsonl", "original content");
        let backup_dir = dir.path().join("backups");

        let backup_path = backup_session(&session, &backup_dir).unwrap();

        assert!(backup_path.exists());
        let backup_content = std::fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, "original content");
        // Filename should contain session name and timestamp
        let name = backup_path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("sess1_"));
        assert!(name.ends_with(".jsonl"));
    }

    #[test]
    fn backup_session_creates_backup_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let session = create_session_file(dir.path(), "sess2.jsonl", "data");
        let backup_dir = dir.path().join("nested").join("backups");

        assert!(!backup_dir.exists());
        let _backup_path = backup_session(&session, &backup_dir).unwrap();
        assert!(backup_dir.exists());
    }

    #[test]
    fn list_backups_returns_session_backups_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backups");

        let _b1 = create_fake_backup(&backup_dir, "sess1", "20260101T100000Z");
        let _b2 = create_fake_backup(&backup_dir, "sess1", "20260101T120000Z");
        let _b3 = create_fake_backup(&backup_dir, "sess1", "20260101T110000Z");

        let backups = list_backups(&backup_dir, "sess1").unwrap();
        assert_eq!(backups.len(), 3);
        // Newest first
        let names: Vec<String> = backups
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(names[0].contains("120000"));
        assert!(names[1].contains("110000"));
        assert!(names[2].contains("100000"));
    }

    #[test]
    fn list_backups_returns_empty_for_no_backups() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();

        let backups = list_backups(&backup_dir, "nonexistent").unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn list_backups_returns_empty_for_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("no-such-dir");

        let backups = list_backups(&backup_dir, "sess1").unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn restore_backup_copies_to_session_path() {
        let dir = tempfile::tempdir().unwrap();
        let session = create_session_file(dir.path(), "session.jsonl", "modified content");
        let backup = create_session_file(dir.path(), "backup.jsonl", "original content");

        restore_backup(&backup, &session).unwrap();

        let content = std::fs::read_to_string(&session).unwrap();
        assert_eq!(content, "original content");
    }

    #[test]
    fn prune_old_backups_with_limit_2_deletes_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backups");

        let _b1 = create_fake_backup(&backup_dir, "sess1", "20260101T100000Z");
        let _b2 = create_fake_backup(&backup_dir, "sess1", "20260101T110000Z");
        let _b3 = create_fake_backup(&backup_dir, "sess1", "20260101T120000Z");
        let _b4 = create_fake_backup(&backup_dir, "sess1", "20260101T130000Z");

        prune_old_backups(&backup_dir, "sess1", 2).unwrap();

        let remaining = list_backups(&backup_dir, "sess1").unwrap();
        assert_eq!(remaining.len(), 2);
        // Only newest 2 remain
        let names: Vec<String> = remaining
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(names[0].contains("130000"));
        assert!(names[1].contains("120000"));
    }

    #[test]
    fn prune_old_backups_with_fewer_than_limit_deletes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backups");

        let b1 = create_fake_backup(&backup_dir, "sess1", "20260101T100000Z");
        let b2 = create_fake_backup(&backup_dir, "sess1", "20260101T110000Z");

        prune_old_backups(&backup_dir, "sess1", 5).unwrap();

        assert!(b1.exists());
        assert!(b2.exists());
    }
}
